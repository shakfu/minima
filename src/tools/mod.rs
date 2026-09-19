//! Four tools, dispatched by enum rather than `dyn Tool`.
//!
//! The freeze pays for itself here: with the set closed at four, an enum removes the trait
//! object, the `async-trait` dependency, and the registry. Adding a fifth tool is three lines.

mod atomic;
mod bash;
mod edit;
mod read;
mod write;

pub use bash::kill_background;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::cancel::Cancel;
use crate::config::TOOL_OUTPUT_CAP;
use crate::provider::{Dialect, anthropic, chat, responses};

/// The result recorded for a call the user cancelled or that never ran because of a cancel.
pub const CANCELLED: &str = "cancelled by the user";

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
            Tool::Bash => {
                "Run a shell command and return its combined output. The command already runs \
                 under `bash -c`; do not wrap it in another shell."
            }
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
    pub async fn call(
        self,
        arguments: &str,
        cancel: &Cancel,
        root: Option<&std::path::Path>,
    ) -> Result<Outcome> {
        let raw = if arguments.trim().is_empty() {
            "{}"
        } else {
            arguments
        };
        let out: Outcome = match self {
            Tool::Read => {
                let mut args: read::Args = parse(raw)?;
                if let Some(root) = root {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                read::call(args).await?.into()
            }
            Tool::Write => {
                let mut args: write::Args = parse(raw)?;
                if let Some(root) = root {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                write::call(args).await?.into()
            }
            Tool::Edit => {
                let mut args: edit::Args = parse(raw)?;
                if let Some(root) = root {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                edit::call(args).await?.into()
            }
            Tool::Bash => bash::call(parse(raw)?, cancel, root).await?,
        };
        Ok(Outcome {
            body: cap(out.body),
            note: out.note,
        })
    }
}

/// Resolve an existing path, or the nearest existing parent of a new path, before a tool opens it.
/// This follows symlinks and removes `..`, so the check is about the filesystem location.
fn confine_path(root: &std::path::Path, raw: &str) -> Result<std::path::PathBuf> {
    let requested = std::path::Path::new(raw);
    let requested = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };

    let mut existing = requested.clone();
    let mut missing = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            bail!("path {raw:?} has no existing parent")
        };
        missing.push(name.to_os_string());
        existing.pop();
    }
    let mut resolved =
        std::fs::canonicalize(&existing).with_context(|| format!("resolving {raw}"))?;
    for name in missing.iter().rev() {
        resolved.push(name);
    }
    if !resolved.starts_with(root) {
        bail!("path {raw:?} is outside root {}", root.display());
    }
    Ok(resolved)
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

/// A fresh directory for one test that removes itself, so tool tests never touch the repo.
#[cfg(test)]
pub struct Scratch(std::path::PathBuf);

#[cfg(test)]
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

#[cfg(test)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
    fn a_path_outside_root_is_refused() {
        let root = Scratch::new("root-boundary");
        let outside = Scratch::new("outside-boundary");
        let path = outside.file("secret.txt");
        std::fs::write(&path, "secret").unwrap();

        let err = confine_path(std::path::Path::new(&root.file(".")), &path).unwrap_err();
        assert!(err.to_string().contains("outside root"), "{err}");
    }

    #[tokio::test]
    async fn no_root_allows_a_path_outside_the_working_root() {
        let root = Scratch::new("no-sandbox-root");
        let outside = Scratch::new("no-sandbox-outside");
        let path = outside.file("secret.txt");
        std::fs::write(&path, "secret\n").unwrap();

        let out = Tool::Read
            .call(
                &serde_json::json!({ "path": path }).to_string(),
                &Cancel::new(),
                None,
            )
            .await
            .unwrap();
        assert!(out.body.contains("secret"));
        drop(root);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_outside_root_is_refused() {
        let root = Scratch::new("root-symlink");
        let outside = Scratch::new("outside-symlink");
        let target = outside.file("secret.txt");
        std::fs::write(&target, "secret").unwrap();
        let link = root.file("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let err = confine_path(std::path::Path::new(&root.file(".")), &link).unwrap_err();
        assert!(err.to_string().contains("outside root"), "{err}");
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
