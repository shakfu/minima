//! Scripted provider. Inside the frozen scope on purpose: without it there is no way to test
//! the agent loop without a network and an API key.
//!
//! The script is a JSON array of `Event`-shaped objects, replayed in order:
//!
//! ```json
//! [{"text": "on it"},
//!  {"tool_call": {"index": 0, "id": "c1", "name": "read", "arguments": "{\"path\":\"x\"}"}},
//!  {"usage": {"prompt_tokens": 12, "completion_tokens": 4}}]
//! ```

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use super::{Error, Event, EventStream, Usage};

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
}

pub struct Mock {
    /// One script, consumed one turn per `[]` group. A Mutex because `stream()` takes `&self`
    /// to match the network provider, and replay position is the only mutable state.
    turns: Mutex<std::vec::IntoIter<Vec<Step>>>,
}

impl Mock {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading the mock script {}", path.display()))?;
        let turns: Vec<Vec<Step>> =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        Ok(Self {
            turns: Mutex::new(turns.into_iter()),
        })
    }

    pub async fn stream(&self) -> Result<EventStream, Error> {
        let steps = self
            .turns
            .lock()
            .expect("mock script lock")
            .next()
            .ok_or_else(|| Error::Other(anyhow!("mock script ran out of turns")))?;

        let mut events: Vec<Result<Event, Error>> = steps
            .into_iter()
            .map(|step| {
                Ok(match step {
                    Step::Text(t) => Event::Text(t),
                    Step::Usage(u) => Event::Usage(Usage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: if u.total_tokens > 0 {
                            u.total_tokens
                        } else {
                            u.prompt_tokens + u.completion_tokens
                        },
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
                })
            })
            .collect();
        events.push(Ok(Event::Done));

        Ok(Box::pin(futures_util::stream::iter(events)))
    }
}
