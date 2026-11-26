//! Where the agent loop's events go.
//!
//! Sync on purpose: every method is a write to an already-open stream, so making the trait async
//! would buy nothing and cost object safety.

use std::time::Duration;

use futures_util::future::LocalBoxFuture;

use crate::provider::Usage;

pub trait Frontend {
    /// A fragment of assistant text, as it streams.
    fn text(&mut self, delta: &str);

    fn tool_start(&mut self, name: &str, arguments: &str);

    /// `note` is set when the tool ran but reported a problem, such as a non-zero exit. `ok` is
    /// false only when the tool itself failed, and `note` is then the error's root cause.
    fn tool_end(&mut self, body: &str, note: Option<&str>, ok: bool);

    fn retry(&mut self, attempt: u32, delay: Duration);

    fn turn_end(&mut self, usage: Usage);

    /// Older messages were replaced by a summary. Both counts are estimated tokens.
    fn compacted(&mut self, before: u32, after: u32);

    fn cancelled(&mut self);

    /// Called after `tool_start`. `None` runs the call; otherwise it runs only if the future
    /// resolves true.
    fn permit(&mut self, _name: &str, _arguments: &str) -> Option<LocalBoxFuture<'static, bool>> {
        None
    }

    /// The model refused the prompt. Called before `run` returns.
    fn refused(&mut self) {}
}
