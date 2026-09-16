//! Four tools, dispatched by enum rather than `dyn Tool`.
//!
//! The freeze pays for itself here: with the set closed at four, an enum removes the trait
//! object, the `async-trait` dependency, and the registry. Adding a fifth tool is three lines,
//! and the README says what that costs.

mod bash;
mod edit;
mod read;
mod write;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::cancel::Cancel;
use crate::config::TOOL_OUTPUT_CAP;
use crate::provider::{Dialect, anthropic, chat, responses};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Read,
    Write,
    Edit,
    Bash,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Read, Tool::Write, Tool::Edit, Tool::Bash];

    pub fn name(self) -> &'static str {
        match self {
            Tool::Read => "read",
            Tool::Write => "write",
            Tool::Edit => "edit",
            Tool::Bash => "bash",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    fn description(self) -> &'static str {
        match self {
            Tool::Read => "Read a UTF-8 text file, returned as numbered lines.",
            Tool::Write => "Write a file, creating or replacing it.",
            Tool::Edit => "Replace an exact string in a file. Fails if it is absent or ambiguous.",
            Tool::Bash => "Run a shell command and return its combined output.",
        }
    }

    fn parameters(self) -> serde_json::Value {
        match self {
            Tool::Read => schema_of::<read::Args>(),
            Tool::Write => schema_of::<write::Args>(),
            Tool::Edit => schema_of::<edit::Args>(),
            Tool::Bash => schema_of::<bash::Args>(),
        }
    }

    /// One entry of the request's `tools` array. The three dialects nest this differently:
    /// Chat wraps it under `function`, Responses keeps it flat, Anthropic renames `parameters`
    /// to `input_schema`.
    pub fn spec(self, dialect: Dialect) -> serde_json::Value {
        let (name, description, parameters) = (self.name(), self.description(), self.parameters());
        match dialect {
            Dialect::Chat => chat::tool_spec(name, description, parameters),
            Dialect::Responses => responses::tool_spec(name, description, parameters),
            Dialect::Messages => anthropic::tool_spec(name, description, parameters),
        }
    }

    /// `arguments` is the raw JSON string the model streamed, parsed here and nowhere else.
    pub async fn call(self, arguments: &str, cancel: &Cancel) -> Result<Outcome> {
        let raw = if arguments.trim().is_empty() {
            "{}"
        } else {
            arguments
        };
        let out: Outcome = match self {
            Tool::Read => read::call(parse(raw)?).await?.into(),
            Tool::Write => write::call(parse(raw)?).await?.into(),
            Tool::Edit => edit::call(parse(raw)?).await?.into(),
            Tool::Bash => bash::call(parse(raw)?, cancel).await?,
        };
        Ok(Outcome {
            body: cap(out.body),
            note: out.note,
        })
    }
}

pub fn specs(dialect: Dialect) -> Vec<serde_json::Value> {
    Tool::ALL.into_iter().map(|t| t.spec(dialect)).collect()
}

fn parse<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    serde_json::from_str(raw).with_context(|| format!("tool arguments were not valid: {raw}"))
}

/// Truncate from the middle: the head says what ran, the tail says how it ended. One `cat` of a
/// build log must not consume the context window.
fn cap(text: String) -> String {
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
fn schema_of<T: schemars::JsonSchema>() -> serde_json::Value {
    let mut schema = schemars::schema_for!(T);
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    schema.to_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tool the model is never told about cannot be called, and a mock cannot catch that.
    /// Each dialect reads the name and schema from a different place.
    #[test]
    fn every_tool_advertises_an_object_schema_in_every_dialect() {
        for tool in Tool::ALL {
            let chat = tool.spec(Dialect::Chat);
            assert_eq!(chat["function"]["name"], tool.name());
            assert_eq!(chat["function"]["parameters"]["type"], "object");

            let responses = tool.spec(Dialect::Responses);
            assert_eq!(responses["name"], tool.name());
            assert_eq!(responses["parameters"]["type"], "object");

            let messages = tool.spec(Dialect::Messages);
            assert_eq!(messages["name"], tool.name());
            assert_eq!(messages["input_schema"]["type"], "object");
            assert!(messages.get("parameters").is_none());

            // OpenAI rejects $schema, and it only costs tokens elsewhere.
            for spec in [
                &chat["function"]["parameters"],
                &responses["parameters"],
                &messages["input_schema"],
            ] {
                assert!(
                    spec.get("$schema").is_none(),
                    "{} leaked $schema",
                    tool.name()
                );
            }
        }
    }

    #[test]
    fn all_four_tools_are_advertised() {
        for dialect in [Dialect::Chat, Dialect::Responses, Dialect::Messages] {
            assert_eq!(specs(dialect).len(), 4);
        }
    }

    #[test]
    fn names_round_trip() {
        for tool in Tool::ALL {
            assert_eq!(Tool::from_name(tool.name()), Some(tool));
        }
        assert_eq!(Tool::from_name("grep"), None);
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
