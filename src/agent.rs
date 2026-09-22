//! The continuation loop, shared by both frontends.
//!
//! Presentation and cancellation arrive through `Frontend` and `Cancel`, so the REPL and the
//! headless path cannot drift apart by each growing their own copy of this loop.

use std::time::Duration;

use anyhow::{Result, bail};
use futures_util::StreamExt;

use crate::cancel::Cancel;
use crate::config::{Bounds, CONTEXT_MARGIN, Config};
use crate::frontend::Frontend;
use crate::prompt::system_prompt;
use crate::provider::{Error, Message, Provider};
use crate::tools::{self, Tool};
use crate::turn::{Assembler, Turn};

/// Retries after the first attempt, so one connection makes at most five requests.
const MAX_RETRIES: u32 = 4;
/// A server that asks for a longer wait than this is reported rather than waited out.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

const CONTEXT_FULL: &str = "not run: the context window is full";
const TRUNCATED: &str = "not run: the response hit the output token limit before the call was \
complete; split the work into smaller calls";

pub struct Agent {
    provider: Provider,
    config: Config,
    bounds: Bounds,
    messages: Vec<Message>,
    tools: Vec<serde_json::Value>,
    /// Total tokens the last turn reported. Checked before each send once it nears the window.
    used: u32,
    on_first_turn: Option<FirstTurn>,
}

type FirstTurn = Box<dyn FnOnce(&Config)>;

impl Agent {
    pub fn with_bounds(provider: Provider, config: Config, bounds: Bounds) -> Self {
        Self {
            provider,
            tools: tools::specs(config.dialect),
            config,
            bounds,
            messages: vec![Message::system(system_prompt())],
            used: 0,
            on_first_turn: None,
        }
    }

    pub fn context_window(&self) -> u32 {
        self.config.context
    }

    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// True when costs come from a price list rather than from the provider.
    pub fn cost_is_estimate(&self) -> bool {
        self.config.pricing.is_some()
    }

    /// Runs once, after the first turn streams to completion: the point at which the provider
    /// has accepted the model.
    pub fn on_first_turn(&mut self, f: impl FnOnce(&Config) + 'static) {
        self.on_first_turn = Some(Box::new(f));
    }

    /// One user prompt and every turn it spawns, until the model stops asking for tools.
    pub async fn run(
        &mut self,
        prompt: &str,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<()> {
        cancel.reset();
        // Refused before sending: a request past the window would be paid for and then fail.
        self.check_context()?;
        self.messages.push(Message::user(prompt));

        for _ in 0..self.config.max_turns {
            let Some(mut turn) = self.one_turn(frontend, cancel).await? else {
                frontend.cancelled();
                return Ok(());
            };
            if let (None, Some(pricing)) = (turn.usage.cost, &self.config.pricing) {
                turn.usage.cost = Some(pricing.cost(&turn.usage));
            }

            frontend.turn_end(turn.usage);
            if turn.usage.total_tokens > 0 {
                self.used = turn.usage.total_tokens;
            }
            if let Some(f) = self.on_first_turn.take() {
                f(&self.config);
            }

            let Turn {
                text,
                mut calls,
                truncated,
                ..
            } = turn;
            // Chat and Responses resend arguments verbatim, and a fragment is not valid JSON.
            if truncated {
                for call in &mut calls {
                    if serde_json::from_str::<serde_json::Map<_, _>>(&call.arguments).is_err() {
                        call.arguments = "{}".into();
                    }
                }
            }
            self.messages.push(Message::assistant(
                (!text.is_empty()).then_some(text),
                calls.clone(),
            ));
            let full = self.check_context();

            if calls.is_empty() {
                full?;
                if truncated {
                    bail!("the response hit the output token limit and is incomplete");
                }
                return Ok(());
            }

            // Every call gets a result, even one that is not run: each dialect rejects a
            // transcript with an unanswered call, which would fail every later request.
            let skip = match (&full, truncated) {
                (Err(_), _) => Some(CONTEXT_FULL),
                (Ok(()), true) => Some(TRUNCATED),
                (Ok(()), false) => None,
            };
            for call in &calls {
                let reason = if cancel.is_cancelled() {
                    Some(tools::CANCELLED)
                } else {
                    skip
                };
                if let Some(reason) = reason {
                    self.messages
                        .push(Message::tool_result(call.id.clone(), reason));
                    continue;
                }
                let name = call.name.as_str();
                frontend.tool_start(name, &call.arguments);

                let outcome = match Tool::from_name(name) {
                    Some(tool) => tool.call(&call.arguments, cancel, &self.bounds).await,
                    None => Err(anyhow::anyhow!("no such tool: {name}")),
                };
                let (ok, body, note) = match outcome {
                    Ok(out) => (true, out.body, out.note),
                    // The user sees only the root cause: the call line already names the target
                    // that the outer context repeats.
                    Err(e) => (
                        false,
                        format!("error: {e:#}"),
                        Some(e.root_cause().to_string()),
                    ),
                };

                frontend.tool_end(&body, note.as_deref(), ok);
                self.messages
                    .push(Message::tool_result(call.id.clone(), body));
            }
            full?;
            // Before the next turn, so a cancelled tool does not cost another request.
            if cancel.is_cancelled() {
                frontend.cancelled();
                return Ok(());
            }
        }

        bail!(
            "stopped after {} turns without a final answer",
            self.config.max_turns
        )
    }

    /// `Ok(None)` means the user cancelled.
    async fn one_turn(&self, frontend: &mut dyn Frontend, cancel: &Cancel) -> Result<Option<Turn>> {
        let Some(mut stream) = self.connect(frontend, cancel).await? else {
            return Ok(None);
        };
        let mut assembler = Assembler::new();

        loop {
            // Biased, so events already buffered are not shown after a cancel.
            let next = tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(None),
                item = stream.next() => item,
            };
            let Some(item) = next else { break };

            match item? {
                crate::provider::Event::Text(text) => {
                    frontend.text(&text);
                    assembler.push(crate::provider::Event::Text(text));
                }
                event => {
                    // Only the stream's own terminator ends the read. A stop reason does not:
                    // Chat sends its usage frame after it.
                    let last = event == crate::provider::Event::Done;
                    assembler.push(event);
                    if last {
                        break;
                    }
                }
            }
        }

        // EOF is not an ending. A proxy or a dropped connection can close the stream after a
        // complete-looking tool call, and running it would act on a request the model never
        // finished making.
        if !assembler.is_done() {
            bail!("the provider closed the stream before the response was complete");
        }
        Ok(Some(assembler.finish()))
    }

    /// Retries the connection only. A stream that dies mid-response is not replayed, because the
    /// partial assistant text has already been shown. `Ok(None)` means the user cancelled.
    async fn connect(
        &self,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Option<crate::provider::EventStream>> {
        let mut attempt = 0;
        loop {
            // A slow first byte can take minutes on a reasoning model; Esc must not wait for it.
            let result = tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(None),
                result = self.provider.stream(&self.config, &self.messages, &self.tools) => result,
            };
            match result {
                Ok(stream) => return Ok(Some(stream)),
                Err(Error::ContextExceeded) => bail!(
                    "the conversation no longer fits in {}'s context window; minima does not \
                     compact, so start a new session",
                    self.config.model
                ),
                Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                    attempt += 1;
                    let delay = e
                        .retry_after()
                        .unwrap_or_else(|| Duration::from_secs(1 << attempt));
                    if delay > MAX_BACKOFF {
                        bail!("{e}; the provider asks for a retry in {}s", delay.as_secs());
                    }
                    frontend.retry(attempt, delay);
                    tokio::select! {
                        () = tokio::time::sleep(delay) => {}
                        () = cancel.cancelled() => return Ok(None),
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// minima reports and refuses. Compaction is not implemented.
    fn check_context(&self) -> Result<()> {
        let used = self.used;
        if used > 0 && used + CONTEXT_MARGIN >= self.config.context {
            bail!(
                "{used} tokens used of {} for {}; start a new session",
                self.config.context,
                self.config.model
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::provider::Usage;
    use crate::tools::Scratch;

    /// Records nothing. With `cancel`, cancels at the first text or tool start, the way Esc would.
    #[derive(Default)]
    struct Quiet {
        cancel: Option<Cancel>,
    }

    impl Frontend for Quiet {
        fn text(&mut self, _: &str) {
            if let Some(cancel) = &self.cancel {
                cancel.cancel();
            }
        }
        fn tool_start(&mut self, _: &str, _: &str) {
            if let Some(cancel) = &self.cancel {
                cancel.cancel();
            }
        }
        fn tool_end(&mut self, _: &str, _: Option<&str>, _: bool) {}
        fn retry(&mut self, _: u32, _: Duration) {}
        fn turn_end(&mut self, _: Usage) {}
        fn cancelled(&mut self) {}
    }

    /// The script holds only the turns a test expects, so any further request fails the run.
    fn agent_with(script: &str) -> Agent {
        agent_with_root(script, std::env::current_dir().unwrap())
    }

    fn agent_with_root(script: &str, root: std::path::PathBuf) -> Agent {
        let mock = crate::provider::mock::Mock::from_script(script).expect("script");
        let bounds = Bounds::new(false, root);
        Agent::with_bounds(Provider::Mock(mock), Config::for_test("m"), bounds)
    }

    fn call(id: &str, name: &str, arguments: serde_json::Value) -> String {
        serde_json::json!({"tool_call": {
            "index": 0, "id": id, "name": name, "arguments": arguments.to_string(),
        }})
        .to_string()
    }

    fn results(agent: &Agent) -> Vec<(String, String)> {
        agent
            .messages
            .iter()
            .filter_map(|m| Some((m.tool_call_id.clone()?, m.content.clone()?)))
            .collect()
    }

    async fn run(agent: &mut Agent) -> Result<()> {
        agent.run("go", &mut Quiet::default(), &Cancel::new()).await
    }

    #[tokio::test]
    async fn a_cancel_between_calls_still_answers_every_call_and_sends_nothing_more() {
        let missing = serde_json::json!({"path": "/nonexistent"});
        let mut agent = agent_with(&format!(
            "[[{}, {}]]",
            call("c1", "read", missing.clone()),
            call("c2", "read", missing).replace("\"index\":0", "\"index\":1")
        ));
        let cancel = Cancel::new();
        let mut frontend = Quiet {
            cancel: Some(cancel.clone()),
        };

        agent
            .run("go", &mut frontend, &cancel)
            .await
            .expect("no second request");

        let results = results(&agent);
        let ids: Vec<_> = results.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["c1", "c2"]);
        assert_eq!(results[1].1, tools::CANCELLED);
    }

    #[tokio::test]
    async fn a_cancel_during_the_last_call_sends_nothing_more() {
        let missing = serde_json::json!({"path": "/nonexistent"});
        let mut agent = agent_with(&format!("[[{}]]", call("c1", "read", missing)));
        let cancel = Cancel::new();
        let mut frontend = Quiet {
            cancel: Some(cancel.clone()),
        };

        agent
            .run("go", &mut frontend, &cancel)
            .await
            .expect("no second request");
        assert_eq!(results(&agent).len(), 1);
    }

    /// The cancelled turn leaves nothing in the transcript, and the session takes the next prompt.
    #[tokio::test]
    async fn a_cancel_mid_stream_drops_the_turn_and_the_session_continues() {
        let mut agent = agent_with(r#"[[{"text": "a"}, {"text": "b"}], [{"text": "c"}]]"#);
        let cancel = Cancel::new();
        let mut frontend = Quiet {
            cancel: Some(cancel.clone()),
        };

        agent.run("go", &mut frontend, &cancel).await.unwrap();
        assert_eq!(agent.messages.len(), 2, "only the system and user messages");

        run(&mut agent).await.expect("the next prompt runs");
        assert_eq!(agent.messages.last().unwrap().content.as_deref(), Some("c"));
    }

    /// The server accepts the connection and never answers, as a stalled provider would.
    #[tokio::test]
    async fn a_cancel_does_not_wait_for_the_response_to_start() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut config = Config::for_test("m");
        config.base_url = format!("http://{}/v1", listener.local_addr().unwrap());
        let http = crate::provider::http::Http::new().unwrap();
        let bounds = Bounds::new(false, std::env::current_dir().unwrap());
        let mut agent = Agent::with_bounds(Provider::Http(http), config, bounds);

        let cancel = Cancel::new();
        tokio::spawn({
            let cancel = cancel.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                cancel.cancel();
            }
        });

        let mut frontend = Quiet::default();
        let run = agent.run("go", &mut frontend, &cancel);
        tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .expect("the cancel ended the wait")
            .expect("a cancel is not an error");
        drop(listener);
    }

    #[tokio::test]
    async fn tool_results_reach_the_transcript_and_the_loop_continues() {
        let dir = Scratch::new("agent-dispatch");
        let path = dir.file("notes.txt");
        std::fs::write(&path, "hello from disk\n").unwrap();
        let mut agent = agent_with_root(
            &format!(
                r#"[[{}, {}], [{{"text": "done"}}]]"#,
                call("c1", "read", serde_json::json!({ "path": path })),
                call("c2", "grep", serde_json::json!({})).replace("\"index\":0", "\"index\":1")
            ),
            std::path::Path::new(&path).parent().unwrap().to_path_buf(),
        );

        run(&mut agent).await.expect("two turns");

        let results = results(&agent);
        assert!(results[0].1.contains("hello from disk"), "{results:?}");
        assert_eq!(results[1].1, "error: no such tool: grep");
    }

    #[tokio::test]
    async fn max_turns_stops_a_model_that_never_answers() {
        let missing = serde_json::json!({"path": "/nonexistent"});
        let mut agent = agent_with(&format!("[[{}]]", call("c1", "read", missing)));
        agent.config.max_turns = 1;

        let err = run(&mut agent).await.expect_err("should stop");
        assert!(err.to_string().contains("stopped after 1 turns"), "{err}");
    }

    /// The turn that fills the window is kept, its calls are answered but not run, and the next
    /// prompt is refused without a request.
    #[tokio::test]
    async fn a_full_context_is_refused_before_the_next_send() {
        let dir = Scratch::new("agent-context");
        let path = dir.file("never.txt");
        let mut agent = agent_with(&format!(
            r#"[[{}, {{"usage": {{"total_tokens": 127000}}}}]]"#,
            call(
                "c1",
                "write",
                serde_json::json!({ "path": path, "content": "x" })
            )
        ));

        let first = run(&mut agent).await.expect_err("full").to_string();
        assert!(first.contains("start a new session"), "{first}");
        assert_eq!(results(&agent)[0].1, CONTEXT_FULL);
        assert!(!std::path::Path::new(&path).exists());

        let second = run(&mut agent).await.expect_err("still full").to_string();
        assert!(second.contains("start a new session"), "{second}");
    }

    #[tokio::test]
    async fn calls_cut_off_at_the_output_limit_are_answered_but_not_run() {
        let dir = Scratch::new("agent-truncated");
        let path = dir.file("never.txt");
        let complete = serde_json::json!({ "path": path, "content": "x" }).to_string();
        let mut agent = agent_with(&format!(
            r#"[[{}, {}, "truncated"], [{{"text": "retried"}}]]"#,
            call(
                "c1",
                "write",
                serde_json::json!({ "path": path, "content": "x" })
            ),
            serde_json::json!({"tool_call": {
                "index": 1, "id": "c2", "name": "write", "arguments": &complete[..10],
            }})
        ));

        run(&mut agent).await.expect("the model gets another turn");
        assert_eq!(results(&agent)[0].1, TRUNCATED);
        assert_eq!(results(&agent)[1].1, TRUNCATED);
        assert!(!std::path::Path::new(&path).exists());

        // A fragment would be resent verbatim and rejected; a complete call keeps its arguments.
        let sent: Vec<_> = agent.messages[2]
            .tool_calls
            .iter()
            .map(|c| c.arguments.as_str())
            .collect();
        assert_eq!(sent, [complete.as_str(), "{}"]);
    }

    #[tokio::test]
    async fn text_cut_off_at_the_output_limit_is_an_error() {
        let mut agent = agent_with(r#"[[{"text": "half an ans"}, "truncated"]]"#);
        let err = run(&mut agent).await.expect_err("incomplete");
        assert!(err.to_string().contains("output token limit"), "{err}");
    }

    /// A stream that ends without a terminal event is a dropped connection, not an answer. The
    /// text has already been shown, but it must not be recorded as the model's reply.
    #[tokio::test]
    async fn a_stream_that_ends_without_a_terminal_event_is_an_error() {
        let mut agent = agent_with(r#"[[{"text": "half an ans"}, "cut"]]"#);
        let err = run(&mut agent).await.expect_err("incomplete");
        assert!(err.to_string().contains("before the response"), "{err}");
        assert_eq!(agent.messages.len(), 2, "only the system and user messages");
    }

    /// The worse half: a call can look complete and still be the front of a longer list.
    #[tokio::test]
    async fn a_tool_call_cut_off_by_a_dropped_stream_is_not_run() {
        let dir = Scratch::new("agent-cut");
        let path = dir.file("never.txt");
        let mut agent = agent_with(&format!(
            r#"[[{}, "cut"]]"#,
            call(
                "c1",
                "write",
                serde_json::json!({ "path": path, "content": "x" })
            )
        ));

        assert!(run(&mut agent).await.is_err());
        assert!(!std::path::Path::new(&path).exists(), "the call ran");
    }

    #[tokio::test]
    async fn the_first_turn_hook_runs_once_and_only_after_a_turn_streams() {
        let calls = Rc::new(Cell::new(0));

        let mut failing = agent_with("[]");
        let counter = Rc::clone(&calls);
        failing.on_first_turn(move |_| counter.set(counter.get() + 1));
        assert!(run(&mut failing).await.is_err());
        assert_eq!(calls.get(), 0);

        let mut working = agent_with(r#"[[{"text": "a"}], [{"text": "b"}]]"#);
        let counter = Rc::clone(&calls);
        working.on_first_turn(move |_| counter.set(counter.get() + 1));
        run(&mut working).await.unwrap();
        run(&mut working).await.unwrap();
        assert_eq!(calls.get(), 1);
    }
}
