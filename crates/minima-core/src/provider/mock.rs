//! Scripted provider, without it there is no way to test
//! the agent loop without a network and an API key.
//!
//! The script is a JSON array of `Event`-shaped objects, replayed in order:
//!
//! ```json
//! [{"text": "on it"},
//!  {"tool_call": {"index": 0, "id": "c1", "name": "read", "arguments": "{\"path\":\"x\"}"}},
//!  {"usage": {"prompt_tokens": 12, "completion_tokens": 4}},
//!  "truncated"]
//! ```
//!
//! `"truncated"` replays a response cut off at the output token limit, `"refused"` one the model
//! refused, and `"cut"` a stream that closes without one: no terminal event follows the steps
//! before it. `{"pause": MS}` waits that many milliseconds before the next step, as a slow
//! provider would.
//!
//! The script may instead be an object from prompt text to turns. Each request then takes the next
//! turn of the latest user prompt, so one script answers several prompts differently.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use serde::Deserialize;

use super::{Error, Event, EventStream, Message, Role, Usage};

/// The script's own shape. The internal `Usage` stays free of serde so no dialect's wire
/// spelling can creep into it.
#[derive(Debug, Deserialize)]
struct ScriptUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
    #[serde(default)]
    cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Step {
    Text(String),
    ToolCall {
        index: usize,
        id: String,
        name: String,
        arguments: String,
    },
    Usage(ScriptUsage),
    /// One whole reasoning block, as a dialect would assemble it.
    Reasoning(serde_json::Value),
    Truncated,
    /// Replays a response the model refused.
    Refused,
    /// Replays a dropped connection: the turn ends here with no terminal event.
    Cut,
    /// Replays a connection reset mid-response: a transport error, then nothing.
    Drop,
    /// Milliseconds to wait before the next step.
    Pause(u64),
}

type Turns = std::vec::IntoIter<Vec<Step>>;

#[derive(Deserialize)]
#[serde(untagged)]
enum Script {
    Sequence(Vec<Vec<Step>>),
    ByPrompt(HashMap<String, Vec<Vec<Step>>>),
}

enum Replay {
    Sequence(Turns),
    ByPrompt(HashMap<String, Turns>),
}

pub struct Mock {
    /// Consumed one turn per `[]` group. A Mutex because `stream()` takes `&self` to match the
    /// network provider, and replay position is the only mutable state.
    turns: Mutex<Replay>,
}

impl Mock {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading the mock script {}", path.display()))?;
        Self::from_script(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn from_script(raw: &str) -> Result<Self> {
        let turns = match serde_json::from_str(raw)? {
            Script::Sequence(turns) => Replay::Sequence(turns.into_iter()),
            Script::ByPrompt(map) => Replay::ByPrompt(
                map.into_iter()
                    .map(|(prompt, turns)| (prompt, turns.into_iter()))
                    .collect(),
            ),
        };
        Ok(Self {
            turns: Mutex::new(turns),
        })
    }

    /// `messages` is the request's conversation; a script keyed by prompt reads its last prompt.
    pub async fn stream(&self, messages: &[Message]) -> Result<EventStream, Error> {
        let steps = match &mut *self.turns.lock().expect("mock script lock") {
            Replay::Sequence(turns) => turns
                .next()
                .ok_or_else(|| Error::Other(anyhow!("mock script ran out of turns")))?,
            Replay::ByPrompt(map) => {
                let prompt = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == Role::User)
                    .and_then(|m| m.content.as_deref())
                    .unwrap_or_default();
                map.get_mut(prompt)
                    .and_then(Iterator::next)
                    .ok_or_else(|| {
                        Error::Other(anyhow!("mock script has no turn left for {prompt:?}"))
                    })?
            }
        };

        let mut cut = false;
        // A pause is carried as the delay before the event after it.
        let mut pause = Duration::ZERO;
        let mut events: Vec<(Duration, Result<Event, Error>)> = Vec::new();
        for step in steps {
            let event = match step {
                Step::Pause(ms) => {
                    pause += Duration::from_millis(ms);
                    continue;
                }
                Step::Cut => {
                    cut = true;
                    continue;
                }
                Step::Drop => {
                    let reset = Err(Error::Transport(anyhow!("connection reset")));
                    events.push((std::mem::take(&mut pause), reset));
                    cut = true;
                    break;
                }
                Step::Text(t) => Event::Text(t),
                Step::Truncated => Event::Truncated,
                Step::Refused => Event::Refused,
                Step::Reasoning(block) => Event::Replay {
                    key: format!("r{}", events.len()),
                    part: block,
                },
                Step::Usage(u) => Event::Usage(Usage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: if u.total_tokens > 0 {
                        u.total_tokens
                    } else {
                        u.prompt_tokens + u.completion_tokens
                    },
                    cost: u.cost,
                    ..Usage::default()
                }),
                Step::ToolCall {
                    index,
                    id,
                    name,
                    arguments,
                } => Event::ToolCallDelta {
                    key: index.to_string(),
                    id: Some(id),
                    name: Some(name),
                    arguments: Some(arguments),
                },
            };
            events.push((std::mem::take(&mut pause), Ok(event)));
        }
        if !cut {
            events.push((pause, Ok(Event::Done)));
        }

        let stream = futures_util::stream::iter(events).then(|(pause, event)| async move {
            if !pause.is_zero() {
                tokio::time::sleep(pause).await;
            }
            event
        });
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn texts(mock: &Mock, messages: &[Message]) -> Vec<String> {
        let events: Vec<_> = mock.stream(messages).await.expect("a turn").collect().await;
        events
            .into_iter()
            .filter_map(|e| match e {
                Ok(Event::Text(t)) => Some(t),
                _ => None,
            })
            .collect()
    }

    /// Each prompt keeps its own place in its turns, and the latest prompt picks them.
    #[tokio::test]
    async fn a_script_keyed_by_prompt_answers_each_prompt_from_its_own_turns() {
        let mock = Mock::from_script(
            r#"{"a": [[{"text": "a1"}], [{"text": "a2"}]], "b": [[{"text": "b1"}]]}"#,
        )
        .unwrap();
        let a = [Message::system("s"), Message::user("a")];
        let b = [Message::user("a"), Message::user("b")];
        assert_eq!(texts(&mock, &a).await, ["a1"]);
        assert_eq!(texts(&mock, &b).await, ["b1"]);
        assert_eq!(texts(&mock, &a).await, ["a2"]);
        let err = mock.stream(&b).await.err().expect("b is spent");
        assert!(err.to_string().contains("\"b\""), "{err}");
    }

    #[tokio::test(start_paused = true)]
    async fn a_pause_delays_the_step_after_it() {
        let mock =
            Mock::from_script(r#"[[{"text": "x"}, {"pause": 5000}, {"text": "y"}]]"#).unwrap();
        let started = tokio::time::Instant::now();
        let mut stream = mock.stream(&[]).await.unwrap();
        assert!(matches!(stream.next().await, Some(Ok(Event::Text(_)))));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(matches!(stream.next().await, Some(Ok(Event::Text(_)))));
        assert!(started.elapsed() >= Duration::from_secs(5));
        assert!(matches!(stream.next().await, Some(Ok(Event::Done))));
    }
}
