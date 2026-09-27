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
use crate::provider::{Error, Message, Provider, Role};
use crate::tools::{self, Tool};
use crate::turn::{Assembler, Turn};

/// Retries after the first attempt, so one connection makes at most five requests.
const MAX_RETRIES: u32 = 4;
/// A server that asks for a longer wait than this is reported rather than waited out.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// For text no provider has counted yet. Code tokenizes denser than this, so the estimate runs low:
/// undercounting sends a request the provider refuses, overcounting refuses one that would fit.
const BYTES_PER_TOKEN: usize = 4;

/// Compaction starts once a request would carry this share of the window. The rest is room for
/// the summary request's output, and for the turns that follow it.
const COMPACT_AT_PERCENT: u64 = 80;
/// The most of the window kept verbatim through a compaction, and never more than half of the
/// conversation, so `/compact` early still frees something.
const KEEP_PERCENT: u64 = 20;
/// Output the summary request must have room for, or compaction is not attempted.
const SUMMARY_ROOM: u32 = 4096;
const SUMMARISE: &str = "Summarise the conversation so far for yourself: it will replace the \
earlier messages, and you will continue the task from the summary and the messages after it. \
Include the user's requests and constraints; decisions made and why; files read, created or \
changed, with the facts from them you still need; commands run and their results; errors not yet \
resolved; and the next steps. Be specific: paths, names, numbers. Reply with text only; do not \
call tools.";
const SUMMARY_HEAD: &str = "The earlier part of this conversation was compacted into this summary:";

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
    /// Estimated tokens appended since `used` was reported. Starts at `overhead`.
    pending: u32,
    /// Estimated tokens of the system prompt and tool schemas, which every request carries.
    overhead: u32,
    on_first_turn: Option<FirstTurn>,
}

type FirstTurn = Box<dyn FnOnce(&Config)>;

enum Attempt {
    Done(Turn),
    Cancelled,
    /// The connection failed mid-response, which a second request may not repeat.
    Dropped(anyhow::Error),
}

/// What a compaction did. Token counts are estimates of what the next request carries.
pub enum Compaction {
    Done {
        before: u32,
        after: u32,
    },
    /// Nothing old enough to summarise, or the summary request itself would not fit.
    Nothing,
    Cancelled,
}

impl Agent {
    pub fn with_bounds(provider: Provider, config: Config, bounds: Bounds) -> Self {
        let system = system_prompt();
        let tools = tools::specs(config.dialect);
        let schemas = serde_json::to_string(&tools).unwrap_or_default();
        let overhead = estimate(&system).saturating_add(estimate(&schemas));
        Self {
            provider,
            tools,
            config,
            bounds,
            messages: vec![Message::system(system)],
            used: 0,
            pending: overhead,
            overhead,
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
        // Named apart from the general check: "start a new session" cannot fix an AGENTS.md.
        if self.overhead.saturating_add(CONTEXT_MARGIN) >= self.config.context {
            bail!(
                "the system prompt and tool schemas take about {} tokens of {}'s {}; shorten \
                 AGENTS.md or the skills list, or raise --context",
                self.overhead,
                self.config.model,
                self.config.context
            );
        }
        // Refused before sending: a request past the window would be paid for and then fail.
        let prompt_tokens = estimate(prompt);
        if !self
            .compact_if_full(prompt_tokens, frontend, cancel)
            .await?
        {
            return Ok(());
        }
        self.check_context(prompt_tokens)?;
        self.pending += prompt_tokens;
        self.messages.push(Message::user(prompt));

        for _ in 0..self.config.max_turns {
            // Tool results can fill the window as surely as a prompt can.
            if !self.compact_if_full(0, frontend, cancel).await? {
                return Ok(());
            }
            self.check_context(0)?;
            let room = self.room();
            let Some(mut turn) = self
                .one_turn(&self.messages, room, frontend, cancel)
                .await?
            else {
                frontend.cancelled();
                return Ok(());
            };
            self.price(&mut turn);

            frontend.turn_end(turn.usage);
            if turn.usage.total_tokens > 0 {
                self.used = turn.usage.total_tokens;
                self.pending = 0;
            }
            if let Some(f) = self.on_first_turn.take() {
                f(&self.config);
            }

            let Turn {
                text,
                calls,
                truncated,
                replay,
                ..
            } = turn;
            // Chat and Responses resend arguments verbatim, and a gateway that translates them to
            // another dialect must parse them. A fragment or malformed object would fail every
            // later request, so the transcript keeps `{}`; the call still runs on what the model
            // sent, so its result names what was wrong.
            let recorded = calls
                .iter()
                .cloned()
                .map(|mut call| {
                    if serde_json::from_str::<serde_json::Map<_, _>>(&call.arguments).is_err() {
                        call.arguments = "{}".into();
                    }
                    call
                })
                .collect();
            // Whitespace alone is no answer, and Anthropic rejects a text block of it.
            let text = (!text.trim().is_empty()).then_some(text);
            let mut message = Message::assistant(text, recorded);
            message.replay = replay;
            self.messages.push(message);
            // A full window after an answer is left to the next prompt, which compacts first.
            if calls.is_empty() {
                if truncated {
                    bail!("the response hit the output token limit and is incomplete");
                }
                return Ok(());
            }

            // Compacted before the calls run rather than refusing them, which would only make the
            // model ask again.
            let mut full = self.check_context(0);
            if full.is_err()
                && !truncated
                && !cancel.is_cancelled()
                && let Compaction::Done { before, after } = self.summarise(frontend, cancel).await?
            {
                frontend.compacted(before, after);
                full = self.check_context(0);
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
                    self.answer(&call.id, reason.to_string());
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
                        tools::cap(format!("error: {e:#}")),
                        Some(e.root_cause().to_string()),
                    ),
                };

                frontend.tool_end(&body, note.as_deref(), ok);
                self.answer(&call.id, body);
            }
            // Before the next turn, so a cancelled tool does not cost another request.
            if cancel.is_cancelled() {
                frontend.cancelled();
                return Ok(());
            }
        }

        bail!(
            "stopped after {} turns without a final answer; the work so far is kept, so a prompt \
             such as \"continue\" resumes it, and --max-turns raises the limit",
            self.config.max_turns
        )
    }

    /// `/compact`: summarises all but the recent turns, whatever the window holds.
    pub async fn compact(
        &mut self,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Compaction> {
        cancel.reset();
        let outcome = self.summarise(frontend, cancel).await?;
        match outcome {
            Compaction::Done { before, after } => frontend.compacted(before, after),
            Compaction::Cancelled => frontend.cancelled(),
            Compaction::Nothing => {}
        }
        Ok(outcome)
    }

    /// Compacts when the next request, `extra` tokens larger, would pass the threshold. False
    /// when the user cancelled it. A compaction that cannot help leaves the refusal to
    /// `check_context`.
    async fn compact_if_full(
        &mut self,
        extra: u32,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<bool> {
        let next = u64::from(self.used.saturating_add(self.pending).saturating_add(extra));
        if next * 100 < u64::from(self.config.context) * COMPACT_AT_PERCENT {
            return Ok(true);
        }
        match self.summarise(frontend, cancel).await? {
            Compaction::Done { before, after } => frontend.compacted(before, after),
            Compaction::Cancelled => {
                frontend.cancelled();
                return Ok(false);
            }
            Compaction::Nothing => {}
        }
        Ok(true)
    }

    /// Replaces the messages before the verbatim tail with the model's summary of them.
    async fn summarise(
        &mut self,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Compaction> {
        let mut cut = self.tail_start();
        // Calls not yet answered stay in the tail: a summary request ending in them is invalid.
        if self
            .messages
            .last()
            .is_some_and(|m| !m.tool_calls.is_empty())
        {
            cut = cut.min(self.messages.len() - 1);
        }
        if cut <= 1 {
            return Ok(Compaction::Nothing);
        }
        let mut request = self.messages[..cut].to_vec();
        request.push(Message::user(SUMMARISE));
        let request_size = self.overhead.saturating_add(sum(&request[1..]));
        if request_size.saturating_add(SUMMARY_ROOM + CONTEXT_MARGIN) >= self.config.context {
            return Ok(Compaction::Nothing);
        }

        let room = self.config.context - request_size;
        let mut quiet = Summarising(frontend);
        let Some(mut turn) = self.one_turn(&request, room, &mut quiet, cancel).await? else {
            return Ok(Compaction::Cancelled);
        };
        self.price(&mut turn);
        quiet.0.turn_end(turn.usage);
        let summary = turn.text.trim();
        if summary.is_empty() {
            bail!("compaction failed: the model returned no summary");
        }

        let summary = Message::user(format!("{SUMMARY_HEAD}\n\n{summary}"));
        let before = self.used.saturating_add(self.pending);
        let after = self
            .overhead
            .saturating_add(size(&summary))
            .saturating_add(sum(&self.messages[cut..]));
        // A short conversation can summarise longer than it is. The history is kept then; the
        // summary request was already paid for, but a larger context would cost more every turn.
        if after >= before {
            return Ok(Compaction::Nothing);
        }
        let tail = self.messages.split_off(cut);
        self.messages.truncate(1);
        self.messages.push(summary);
        self.messages.extend(tail);
        self.used = 0;
        self.pending = after;
        Ok(Compaction::Done { before, after })
    }

    /// Where the verbatim tail starts: the earliest turn boundary whose suffix fits the budget.
    /// A boundary is a prompt or an assistant message, never a tool result, so a call is never
    /// kept without its answer or summarised apart from it. The length when nothing fits.
    fn tail_start(&self) -> usize {
        let conversation = sum(&self.messages[1..]);
        let budget = u64::from(self.config.context) * KEEP_PERCENT / 100;
        let budget = u32::try_from(budget)
            .unwrap_or(u32::MAX)
            .min(conversation / 2);
        let (mut start, mut kept) = (self.messages.len(), 0u32);
        // From 2: the first message after the system prompt is always summarised.
        for i in (2..self.messages.len()).rev() {
            kept = kept.saturating_add(size(&self.messages[i]));
            if kept > budget {
                break;
            }
            if self.messages[i].role != Role::Tool {
                start = i;
            }
        }
        start
    }

    /// Fills in the cost from the price list when the provider reported none.
    fn price(&self, turn: &mut Turn) {
        if let (None, Some(pricing)) = (turn.usage.cost, &self.config.pricing) {
            turn.usage.cost = Some(pricing.cost(&turn.usage));
        }
    }

    /// `Ok(None)` means the user cancelled. A response that dies mid-way is requested again: no
    /// call from it has run and nothing of it is recorded, so only the text shown repeats.
    async fn one_turn(
        &self,
        messages: &[Message],
        room: u32,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Option<Turn>> {
        let mut attempt = 0;
        loop {
            let dropped = match self.attempt(messages, room, frontend, cancel).await? {
                Attempt::Done(turn) => return Ok(Some(turn)),
                Attempt::Cancelled => return Ok(None),
                Attempt::Dropped(e) => e,
            };
            if attempt >= MAX_RETRIES {
                bail!(
                    "{dropped:#}; nothing from this response was run or recorded, so a prompt \
                     such as \"continue\" retries it"
                );
            }
            attempt += 1;
            let delay = Duration::from_secs(1 << attempt);
            frontend.retry(attempt, delay);
            tokio::select! {
                () = tokio::time::sleep(delay) => {}
                () = cancel.cancelled() => return Ok(None),
            }
        }
    }

    /// One request and its response.
    async fn attempt(
        &self,
        messages: &[Message],
        room: u32,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Attempt> {
        let Some(mut stream) = self.connect(messages, room, frontend, cancel).await? else {
            return Ok(Attempt::Cancelled);
        };
        let mut assembler = Assembler::new();
        let mut shown = Visible::default();

        loop {
            // Biased, so events already buffered are not shown after a cancel.
            let next = tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(Attempt::Cancelled),
                item = stream.next() => item,
            };
            let Some(item) = next else { break };

            match item {
                Err(e) if e.is_retryable() => return Ok(Attempt::Dropped(e.into())),
                Err(e) => return Err(e.into()),
                Ok(crate::provider::Event::Text(text)) => {
                    if let Some(text) = shown.pass(&text) {
                        frontend.text(&text);
                    }
                    assembler.push(crate::provider::Event::Text(text));
                }
                Ok(crate::provider::Event::Break) => {
                    shown.pass(crate::turn::PARAGRAPH);
                    assembler.push(crate::provider::Event::Break);
                }
                Ok(event) => {
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
            return Ok(Attempt::Dropped(anyhow::anyhow!(
                "the provider closed the stream before the response was complete"
            )));
        }
        assembler.finish().map(Attempt::Done)
    }

    /// Retries until a response starts; `one_turn` retries one that dies after. `Ok(None)` means
    /// the user cancelled.
    async fn connect(
        &self,
        messages: &[Message],
        room: u32,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Option<crate::provider::EventStream>> {
        let mut attempt = 0;
        loop {
            // A slow first byte can take minutes on a reasoning model; Esc must not wait for it.
            let result = tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(None),
                result = self.provider.stream(&self.config, messages, &self.tools, room) => result,
            };
            match result {
                Ok(stream) => return Ok(Some(stream)),
                Err(Error::ContextExceeded) => bail!(
                    "the conversation no longer fits in {}'s context window, though the \
                     estimate said it would; run /compact, or start a new session",
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

    /// Estimated tokens the window has left for the response.
    fn room(&self) -> u32 {
        let used = self.used.saturating_add(self.pending);
        self.config.context.saturating_sub(used)
    }

    fn answer(&mut self, call_id: &str, body: String) {
        self.pending = self.pending.saturating_add(estimate(&body));
        self.messages.push(Message::tool_result(call_id, body));
    }

    /// The hard limit, after compaction has had its chance. `extra` is text about to be appended.
    fn check_context(&self, extra: u32) -> Result<()> {
        let unreported = self.pending.saturating_add(extra);
        let used = self.used.saturating_add(unreported);
        if used.saturating_add(CONTEXT_MARGIN) >= self.config.context {
            let about = if unreported > 0 { "about " } else { "" };
            bail!(
                "{about}{used} tokens used of {} for {}; start a new session",
                self.config.context,
                self.config.model
            );
        }
        Ok(())
    }
}

fn estimate(text: &str) -> u32 {
    u32::try_from(text.len() / BYTES_PER_TOKEN).unwrap_or(u32::MAX)
}

/// Estimated tokens of one message as sent: its text and its calls.
fn size(m: &Message) -> u32 {
    let calls = m
        .tool_calls
        .iter()
        .map(|c| estimate(&c.name).saturating_add(estimate(&c.arguments)))
        .fold(0u32, u32::saturating_add);
    estimate(m.content.as_deref().unwrap_or_default()).saturating_add(calls)
}

fn sum(messages: &[Message]) -> u32 {
    messages.iter().map(size).fold(0, u32::saturating_add)
}

/// Holds whitespace back until visible text follows it. Some models send `"\n\n"` alone before
/// their tool calls, which printed as runs of blank rows between tool lines. Leading newlines of a
/// turn are dropped; indentation is kept, since an answer can open with an indented block.
#[derive(Default)]
struct Visible {
    started: bool,
    held: String,
}

impl Visible {
    /// The text to show now, if any.
    fn pass(&mut self, delta: &str) -> Option<String> {
        let delta = if self.started {
            delta
        } else {
            delta.trim_start_matches(['\n', '\r'])
        };
        let body = delta.trim_end();
        if body.is_empty() {
            if self.started {
                self.held.push_str(delta);
            }
            return None;
        }
        self.started = true;
        let out = std::mem::take(&mut self.held) + body;
        self.held = delta[body.len()..].to_string();
        Some(out)
    }
}

/// The summary request's frontend: its text is not an answer and is not shown. Retries are.
struct Summarising<'a>(&'a mut dyn Frontend);

impl Frontend for Summarising<'_> {
    fn text(&mut self, _: &str) {}
    fn tool_start(&mut self, _: &str, _: &str) {}
    fn tool_end(&mut self, _: &str, _: Option<&str>, _: bool) {}
    fn retry(&mut self, attempt: u32, delay: Duration) {
        self.0.retry(attempt, delay);
    }
    fn turn_end(&mut self, _: crate::provider::Usage) {}
    fn compacted(&mut self, _: u32, _: u32) {}
    fn cancelled(&mut self) {}
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
        fn compacted(&mut self, _: u32, _: u32) {}
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

    /// The turn that fills the window is kept, the messages before it are summarised, and only
    /// then do its calls run: refusing them would only make the model ask again.
    #[tokio::test]
    async fn a_call_that_fills_the_window_runs_after_compaction() {
        let dir = Scratch::new("agent-context");
        let path = dir.file("never.txt");
        let mut agent = agent_with(&format!(
            r#"[[{}, {{"usage": {{"total_tokens": 127000}}}}],
                [{{"text": "SUMMARY"}}],
                [{{"text": "done"}}]]"#,
            call(
                "c1",
                "write",
                serde_json::json!({ "path": path, "content": "x" })
            )
        ));

        run(&mut agent).await.expect("compacted, then answered");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
        assert_eq!(
            agent.messages[2].tool_calls[0].id, "c1",
            "the call is kept verbatim"
        );
        let summary = agent.messages[1].content.as_deref().unwrap();
        assert!(
            summary.starts_with(SUMMARY_HEAD) && summary.ends_with("SUMMARY"),
            "{summary}"
        );
        assert_eq!(
            agent.messages.last().unwrap().content.as_deref(),
            Some("done")
        );
    }

    /// The kept tail starts at a turn boundary, so the last call keeps its result and the large
    /// early result is what gets summarised.
    #[test]
    fn the_verbatim_tail_starts_at_a_turn_boundary() {
        let mut agent = agent_with("[]");
        let read = |id: &str| crate::provider::ToolCall {
            id: id.into(),
            name: "read".into(),
            arguments: r#"{"path": "f"}"#.into(),
        };
        agent.messages.extend([
            Message::user("first"),
            Message::assistant(None, vec![read("c1")]),
            Message::tool_result("c1", "x".repeat(40_000)),
            Message::assistant(Some("answer".into()), vec![]),
            Message::user("second"),
            Message::assistant(None, vec![read("c2")]),
            Message::tool_result("c2", "short"),
        ]);
        assert_eq!(
            agent.tail_start(),
            4,
            "from the answer after the large result"
        );
    }

    /// Summarising three short messages yields more text than they hold.
    #[tokio::test]
    async fn a_summary_no_smaller_than_the_history_is_discarded() {
        let long = "a summary longer than the conversation it replaces ".repeat(20);
        let mut agent = agent_with(&format!(r#"[[{{"text": "hi"}}], [{{"text": "{long}"}}]]"#));
        run(&mut agent).await.unwrap();
        let kept = agent.messages.len();

        let outcome = agent
            .compact(&mut Quiet::default(), &Cancel::new())
            .await
            .unwrap();
        assert!(matches!(outcome, Compaction::Nothing));
        assert_eq!(agent.messages.len(), kept);
        assert_eq!(agent.messages[1].content.as_deref(), Some("go"));
    }

    #[tokio::test]
    async fn compact_with_no_history_does_nothing_and_sends_nothing() {
        let mut agent = agent_with("[]");
        let outcome = agent
            .compact(&mut Quiet::default(), &Cancel::new())
            .await
            .unwrap();
        assert!(matches!(outcome, Compaction::Nothing));
        assert_eq!(agent.messages.len(), 1);
    }

    /// A file of `bytes` under a scratch directory, in lines, so `read` returns all of it.
    fn big_file(dir: &Scratch, bytes: usize) -> String {
        let path = dir.file("big.txt");
        std::fs::write(&path, format!("{}\n", "x".repeat(79)).repeat(bytes / 80)).unwrap();
        path
    }

    /// 3000 reported, plus 32 KiB of capped output at 4 bytes a token, plus the margin, is past
    /// 12000. The script has no second turn, so a request would fail with another error.
    #[tokio::test]
    async fn tool_results_that_fill_the_window_are_refused_before_the_next_send() {
        let dir = Scratch::new("agent-results");
        let path = big_file(&dir, 40_000);
        let mut agent = agent_with(&format!(
            r#"[[{}, {{"usage": {{"total_tokens": 3000}}}}]]"#,
            call("c1", "read", serde_json::json!({ "path": path }))
        ));
        agent.config.context = 12_000;

        let err = run(&mut agent).await.expect_err("full").to_string();
        assert!(
            err.contains("about") && err.contains("start a new session"),
            "{err}"
        );
        assert!(results(&agent)[0].1.len() > 30_000, "the call ran");
    }

    #[tokio::test]
    async fn a_prompt_too_large_for_the_window_is_refused_before_it_is_sent() {
        let mut agent = agent_with("[]");
        agent.config.context = 12_000;

        let err = agent
            .run(&"x".repeat(48_000), &mut Quiet::default(), &Cancel::new())
            .await
            .expect_err("full")
            .to_string();
        assert!(err.contains("start a new session"), "{err}");
        assert_eq!(agent.messages.len(), 1, "only the system message");
    }

    /// Two reads of about 8k tokens each would pass 20000 as estimates, but the provider's count
    /// after the first replaces it.
    #[tokio::test]
    async fn a_reported_count_replaces_the_estimate() {
        let dir = Scratch::new("agent-reported");
        let path = big_file(&dir, 40_000);
        let read = call("c1", "read", serde_json::json!({ "path": path }));
        let mut agent = agent_with(&format!(
            r#"[[{read}, {{"usage": {{"total_tokens": 3000}}}}],
                [{}, {{"usage": {{"total_tokens": 4000}}}}],
                [{{"text": "done"}}]]"#,
            read.replace("c1", "c2")
        ));
        agent.config.context = 20_000;

        run(&mut agent).await.expect("fits");
        assert_eq!(
            agent.messages.last().unwrap().content.as_deref(),
            Some("done")
        );
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

    /// A complete turn can carry malformed arguments too. The call runs and reports the error,
    /// and the transcript keeps `{}`, which every dialect and gateway accepts.
    #[tokio::test]
    async fn malformed_arguments_are_answered_and_not_resent() {
        let raw = r#"{"path": "x", "content": "unterminated"#;
        let mut agent = agent_with(&format!(
            r#"[[{}], [{{"text": "retried"}}]]"#,
            serde_json::json!({"tool_call": {
                "index": 0, "id": "c1", "name": "write", "arguments": raw,
            }})
        ));

        run(&mut agent).await.expect("the model gets another turn");
        let results = results(&agent);
        assert!(
            results[0]
                .1
                .starts_with("error: tool arguments were not valid"),
            "{results:?}"
        );
        assert!(!results[0].1.contains("unterminated"), "{results:?}");
        assert_eq!(agent.messages[2].tool_calls[0].arguments, "{}");
    }

    /// Before the first response nothing is reported, so the fixed part of every request must
    /// be estimated, or a large AGENTS.md sends a request the provider refuses.
    #[tokio::test]
    async fn the_system_prompt_and_schemas_count_before_the_first_response() {
        let mut agent = agent_with("[]");
        assert!(agent.overhead > 0);
        agent.config.context = agent.overhead + CONTEXT_MARGIN;

        let err = run(&mut agent).await.expect_err("full").to_string();
        assert!(err.contains("system prompt and tool schemas"), "{err}");
        assert_eq!(agent.messages.len(), 1, "only the system message");
    }

    /// The next request of the turn must carry the reasoning, or the provider drops it.
    #[tokio::test]
    async fn reasoning_is_kept_on_the_assistant_message() {
        let block = serde_json::json!({"type": "thinking", "thinking": "", "signature": "S"});
        let mut agent = agent_with(&format!(
            r#"[[{{"reasoning": {block}}}, {}], [{{"text": "done"}}]]"#,
            call("c1", "read", serde_json::json!({"path": "/nonexistent"}))
        ));
        run(&mut agent).await.unwrap();
        assert_eq!(agent.messages[2].replay, [block]);
    }

    #[test]
    fn whitespace_is_shown_only_between_visible_text() {
        let mut v = Visible::default();
        let shown: Vec<_> = ["\n\n", "  ", "  code", "\n\n", "", "next\n", "\n"]
            .iter()
            .map(|d| v.pass(d))
            .collect();
        assert_eq!(
            shown,
            [
                None,
                None,
                Some("  code".into()),
                None,
                None,
                Some("\n\nnext".into()),
                None
            ]
        );
    }

    /// A turn of only whitespace before its calls shows nothing and records no text.
    #[tokio::test]
    async fn a_whitespace_only_turn_is_neither_shown_nor_recorded() {
        #[derive(Default)]
        struct Shown(String);
        impl Frontend for Shown {
            fn text(&mut self, t: &str) {
                self.0.push_str(t);
            }
            fn tool_start(&mut self, _: &str, _: &str) {}
            fn tool_end(&mut self, _: &str, _: Option<&str>, _: bool) {}
            fn retry(&mut self, _: u32, _: Duration) {}
            fn turn_end(&mut self, _: Usage) {}
            fn compacted(&mut self, _: u32, _: u32) {}
            fn cancelled(&mut self) {}
        }
        let mut agent = agent_with(&format!(
            r#"[[{{"text": "\n\n"}}, {}], [{{"text": "\ndone"}}]]"#,
            call("c1", "read", serde_json::json!({"path": "/nonexistent"}))
        ));
        let mut shown = Shown::default();
        agent.run("go", &mut shown, &Cancel::new()).await.unwrap();
        assert_eq!(shown.0, "done");
        assert_eq!(agent.messages[2].content, None);
    }

    #[tokio::test]
    async fn text_cut_off_at_the_output_limit_is_an_error() {
        let mut agent = agent_with(r#"[[{"text": "half an ans"}, "truncated"]]"#);
        let err = run(&mut agent).await.expect_err("incomplete");
        assert!(err.to_string().contains("output token limit"), "{err}");
    }

    /// A stream that ends without a terminal event is a dropped connection, not an answer. The
    /// text has already been shown, but it must not be recorded as the model's reply. Once the
    /// retries are spent, the error says the session can go on.
    #[tokio::test(start_paused = true)]
    async fn a_stream_that_ends_without_a_terminal_event_is_an_error() {
        let cut = r#"[{"text": "half an ans"}, "cut"]"#;
        let mut agent = agent_with(&format!("[{}]", [cut; 5].join(",")));
        let err = run(&mut agent).await.expect_err("incomplete").to_string();
        assert!(err.contains("before the response"), "{err}");
        assert!(err.contains("\"continue\""), "{err}");
        assert_eq!(agent.messages.len(), 2, "only the system and user messages");
    }

    /// Nothing of a dropped response ran, so the retry is the first time its calls run.
    #[tokio::test(start_paused = true)]
    async fn a_response_dropped_mid_way_is_requested_again_and_its_calls_run_once() {
        #[derive(Default)]
        struct Retries(u32);
        impl Frontend for Retries {
            fn text(&mut self, _: &str) {}
            fn tool_start(&mut self, _: &str, _: &str) {}
            fn tool_end(&mut self, _: &str, _: Option<&str>, _: bool) {}
            fn retry(&mut self, _: u32, _: Duration) {
                self.0 += 1;
            }
            fn turn_end(&mut self, _: Usage) {}
            fn compacted(&mut self, _: u32, _: u32) {}
            fn cancelled(&mut self) {}
        }
        let dir = Scratch::new("agent-dropped");
        let path = dir.file("once.txt");
        let write = call(
            "c1",
            "edit",
            serde_json::json!({ "path": path, "old": "a", "new": "aa" }),
        );
        std::fs::write(&path, "a").unwrap();
        let mut agent = agent_with(&format!(
            r#"[[{{"text": "writing"}}, {write}, "drop"],
                [{{"text": "writing"}}, {write}],
                [{{"text": "done"}}]]"#
        ));
        let mut frontend = Retries::default();
        agent
            .run("go", &mut frontend, &Cancel::new())
            .await
            .unwrap();
        assert_eq!(frontend.0, 1);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "aa",
            "the edit ran once"
        );
        let assistants = agent
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .count();
        assert_eq!(assistants, 2, "the dropped response is not recorded");
    }

    /// The worse half: a call can look complete and still be the front of a longer list.
    #[tokio::test(start_paused = true)]
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

    /// The named call is not run either: it may be half of a pair.
    #[tokio::test]
    async fn a_nameless_call_in_a_complete_response_is_an_error_and_nothing_runs() {
        let dir = Scratch::new("agent-nameless");
        let path = dir.file("never.txt");
        let mut agent = agent_with(&format!(
            "[[{}, {}]]",
            call(
                "c1",
                "write",
                serde_json::json!({ "path": path, "content": "x" })
            ),
            call("c2", "", serde_json::json!({})).replace("\"index\":0", "\"index\":1")
        ));

        let err = run(&mut agent).await.expect_err("nameless").to_string();
        assert!(err.contains("no name"), "{err}");
        assert_eq!(agent.messages.len(), 2, "only the system and user messages");
        assert!(!std::path::Path::new(&path).exists(), "the named call ran");
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
