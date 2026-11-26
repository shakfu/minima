//! What the agent loop needs from a toolset, and helpers every toolset uses.

use anyhow::{Context, Result};
use futures_util::future::LocalBoxFuture;
use serde::Deserialize;

use crate::cancel::Cancel;
use crate::config::TOOL_OUTPUT_CAP;
use crate::provider::{Dialect, anthropic, chat, responses};

/// The result recorded for a call the user cancelled or that never ran because of a cancel.
pub const CANCELLED: &str = "cancelled by the user";

/// The result recorded for a call the user did not permit.
pub const DENIED: &str = "not run: the user denied permission for this call";

/// What a tool produced. `note` is set when the tool ran but the work it did reported a problem,
/// such as a command exiting non-zero. That is not a tool failure -- the model needs the output
/// either way -- but the user should still see why a turn took two attempts.
#[derive(Debug, Default)]
pub struct Outcome {
    pub body: String,
    pub note: Option<String>,
}

impl From<String> for Outcome {
    fn from(body: String) -> Self {
        Self { body, note: None }
    }
}

/// One tool as advertised, before a dialect shapes it.
#[derive(Debug, Clone)]
pub struct Spec {
    pub name: String,
    pub description: String,
    /// A JSON Schema object; see `schema_of`.
    pub parameters: serde_json::Value,
}

impl Spec {
    /// One entry of the request's `tools` array. Chat wraps it under `function`, Responses keeps
    /// it flat, Anthropic renames `parameters` to `input_schema`.
    pub fn wire(&self, dialect: Dialect) -> serde_json::Value {
        let (name, description, parameters) =
            (&*self.name, &*self.description, self.parameters.clone());
        match dialect {
            Dialect::Chat => chat::tool_spec(name, description, parameters),
            Dialect::Responses => responses::tool_spec(name, description, parameters),
            Dialect::Messages => anthropic::tool_spec(name, description, parameters),
        }
    }
}

/// The tools an agent may call. Fixed for the agent's life: the schemas are sent with every
/// request and counted once.
pub trait Tools {
    fn specs(&self) -> Vec<Spec>;

    /// Runs one call. `arguments` is the raw JSON object the model streamed; see `parse`. A name
    /// `specs` did not list is an error. The body is capped by the caller.
    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: &'a str,
        cancel: &'a Cancel,
    ) -> LocalBoxFuture<'a, Result<Outcome>>;
}

/// The arguments are already in the transcript, so the error names only what is wrong with them.
/// Empty arguments parse as `{}`.
pub fn parse<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    let raw = if raw.trim().is_empty() { "{}" } else { raw };
    serde_json::from_str(raw).context("tool arguments were not valid")
}

/// Drops `work` on a cancel, so the work must be safe to drop at any await.
pub async fn or_cancelled(
    cancel: &Cancel,
    work: impl Future<Output = Result<String>>,
) -> Result<Outcome> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Ok(CANCELLED.to_string().into()),
        body = work => Ok(body?.into()),
    }
}

/// Truncate from the middle: the head says what ran, the tail says how it ended. One `cat` of a
/// build log must not consume the context window.
pub fn cap(text: String) -> String {
    if text.len() <= TOOL_OUTPUT_CAP {
        return text;
    }
    let half = TOOL_OUTPUT_CAP / 2;
    let head = floor_boundary(&text, half);
    let tail = ceil_boundary(&text, text.len() - half);
    let dropped = tail - head;
    format!(
        "{}\n... {dropped} bytes elided ...\n{}",
        &text[..head],
        &text[tail..]
    )
}

fn floor_boundary(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_boundary(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// OpenAI rejects `$schema`, and `title` only adds tokens.
pub fn schema_of<T: schemars::JsonSchema>() -> serde_json::Value {
    let mut schema = schemars::schema_for!(T);
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    schema.to_value()
}

/// A fresh directory for one test that removes itself, so tool tests never touch the repo.
#[cfg(any(test, feature = "testing"))]
pub struct Scratch(std::path::PathBuf);

#[cfg(any(test, feature = "testing"))]
impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("minima-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }

    /// A path inside the directory, as the string a tool argument carries.
    pub fn file(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}

#[cfg(any(test, feature = "testing"))]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_cancel_ends_work_that_is_still_running() {
        let cancel = Cancel::new();
        cancel.cancel();
        let out = or_cancelled(&cancel, std::future::pending()).await.unwrap();
        assert_eq!(out.body, CANCELLED);
    }

    /// The transcript already holds the arguments; a failed 30 KiB `write` must not repeat them.
    #[test]
    fn invalid_arguments_are_not_echoed_back() {
        let raw = format!("{{\"path\": \"x\", \"content\": \"{}", "y".repeat(1000));
        let err = parse::<serde_json::Value>(&raw).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.starts_with("tool arguments were not valid"), "{text}");
        assert!(!text.contains("yyyy"), "{text}");
    }

    #[test]
    fn empty_arguments_are_an_empty_object() {
        let v: serde_json::Value = parse("  ").unwrap();
        assert_eq!(v, serde_json::json!({}));
    }

    #[test]
    fn cap_keeps_both_ends_and_utf8() {
        let text = format!("{}{}", "a".repeat(TOOL_OUTPUT_CAP), "\u{00e9}z");
        let out = cap(text);
        assert!(out.len() < TOOL_OUTPUT_CAP + 64);
        assert!(out.starts_with('a') && out.ends_with('z'));
        assert!(out.contains("bytes elided"));
    }

    #[test]
    fn cap_leaves_short_output_alone() {
        assert_eq!(cap("hi".into()), "hi");
    }
}
