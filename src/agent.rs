//! The continuation loop, shared by both frontends.
//!
//! Presentation and cancellation arrive through `Frontend` and `Cancel`, so the REPL and the
//! headless path cannot drift apart by each growing their own copy of this loop.

use std::time::Duration;

use anyhow::{Result, bail};
use futures_util::StreamExt;

use crate::cancel::Cancel;
use crate::config::{CONTEXT_MARGIN, Config};
use crate::frontend::Frontend;
use crate::provider::{Error, Message, Provider, Usage};
use crate::tools::{self, Tool};
use crate::turn::Assembler;

const MAX_ATTEMPTS: u32 = 4;
const SYSTEM: &str = "You are minima, a coding agent. Use the tools to inspect and change files. \
Be terse. State what you did; do not narrate what you are about to do.";

/// Facts about the machine, so the model does not have to guess at it. Without this it reaches
/// for GNU flags on a BSD userland and burns a turn discovering the mistake.
///
/// Three fields, not hax's six: the working directory, the platform, and the shell. Home
/// directory and model name are not worth their tokens, and a git root needs a walk up the tree.
fn system_prompt() -> String {
    let mut prompt = String::from(SYSTEM);
    prompt.push_str("\n\n# Environment\n\n");

    if let Ok(cwd) = std::env::current_dir() {
        prompt.push_str(&format!("- Working directory: {}\n", cwd.display()));
    }
    prompt.push_str(&format!(
        "- Operating system: {} ({})\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    if let Ok(shell) = std::env::var("SHELL") {
        prompt.push_str(&format!("- Command shell: {shell}\n"));
    }
    prompt
}

pub struct Agent {
    provider: Provider,
    config: Config,
    messages: Vec<Message>,
    tools: Vec<serde_json::Value>,
}

impl Agent {
    pub fn new(provider: Provider, config: Config) -> Self {
        Self {
            provider,
            tools: tools::specs(config.dialect),
            config,
            messages: vec![Message::system(system_prompt())],
        }
    }

    /// One user prompt and every turn it spawns, until the model stops asking for tools.
    pub async fn run(
        &mut self,
        prompt: &str,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<()> {
        cancel.reset();
        self.messages.push(Message::user(prompt));

        for _ in 0..self.config.max_turns {
            let Some(turn) = self.one_turn(frontend, cancel).await? else {
                frontend.cancelled();
                return Ok(());
            };

            self.check_context(turn.usage)?;
            frontend.turn_end(turn.usage);
            self.messages.push(Message::assistant(
                (!turn.text.is_empty()).then(|| turn.text.clone()),
                turn.calls.clone(),
            ));

            if !turn.wants_tools() {
                return Ok(());
            }

            for call in &turn.calls {
                if cancel.is_cancelled() {
                    frontend.cancelled();
                    return Ok(());
                }
                let name = call.name.as_str();
                frontend.tool_start(name, &call.arguments);

                let outcome = match Tool::from_name(name) {
                    Some(tool) => tool.call(&call.arguments, cancel).await,
                    None => Err(anyhow::anyhow!("no such tool: {name}")),
                };
                let (ok, body, note) = match outcome {
                    Ok(out) => (true, out.body, out.note),
                    Err(e) => (false, format!("error: {e:#}"), None),
                };

                frontend.tool_end(name, &body, note.as_deref(), ok);
                self.messages
                    .push(Message::tool_result(call.id.clone(), body));
            }
        }

        bail!(
            "stopped after {} turns without a final answer",
            self.config.max_turns
        )
    }

    /// `Ok(None)` means the user cancelled mid-stream.
    async fn one_turn(
        &self,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<Option<crate::turn::Turn>> {
        let mut stream = self.connect(frontend, cancel).await?;
        let mut assembler = Assembler::new();

        loop {
            let next = tokio::select! {
                item = stream.next() => item,
                () = cancel.cancelled() => return Ok(None),
            };
            let Some(item) = next else { break };

            match item? {
                crate::provider::Event::Text(text) => {
                    frontend.text(&text);
                    assembler.push(crate::provider::Event::Text(text));
                }
                event => {
                    assembler.push(event);
                    if assembler.is_done() {
                        break;
                    }
                }
            }
        }

        Ok(Some(assembler.finish()))
    }

    /// Retries the connection only. A stream that dies mid-response is not replayed, because the
    /// partial assistant text has already been shown.
    async fn connect(
        &self,
        frontend: &mut dyn Frontend,
        cancel: &Cancel,
    ) -> Result<crate::provider::EventStream> {
        let mut attempt = 0;
        loop {
            match self
                .provider
                .stream(&self.config, &self.messages, &self.tools)
                .await
            {
                Ok(stream) => return Ok(stream),
                Err(Error::ContextExceeded) => bail!(
                    "the conversation no longer fits in {}'s context window; minima does not \
                     compact, so start a new session",
                    self.config.model
                ),
                Err(e) if e.is_retryable() && attempt < MAX_ATTEMPTS => {
                    attempt += 1;
                    let delay = e
                        .retry_after()
                        .unwrap_or_else(|| Duration::from_secs(1 << attempt));
                    frontend.retry(attempt, delay);
                    tokio::select! {
                        () = tokio::time::sleep(delay) => {}
                        () = cancel.cancelled() => bail!("cancelled while backing off"),
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// minima reports and refuses. Compaction is outside the frozen scope.
    fn check_context(&self, usage: Usage) -> Result<()> {
        let used = usage.total_tokens;
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
    use super::*;

    #[test]
    fn the_system_prompt_states_the_platform_and_place() {
        let prompt = system_prompt();
        assert!(prompt.starts_with("You are minima"));
        assert!(prompt.contains("# Environment"));
        assert!(prompt.contains(std::env::consts::OS));
        assert!(prompt.contains(std::env::consts::ARCH));

        let cwd = std::env::current_dir().expect("a working directory");
        assert!(prompt.contains(&cwd.display().to_string()));
    }

    /// An unset SHELL must drop the line rather than print an empty one.
    #[test]
    fn an_unknown_shell_is_omitted_not_blank() {
        let prompt = system_prompt();
        assert!(!prompt.contains("- Command shell: \n"));
    }
}
