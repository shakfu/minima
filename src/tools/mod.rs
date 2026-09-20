//! Four tools, dispatched by enum rather than `dyn Tool`.
//!
//! The freeze pays for itself here: with the set closed at four, an enum removes the trait
//! object, the `async-trait` dependency, and the registry. Adding a fifth tool is three lines.

mod atomic;
mod bash;
mod edit;
mod read;
mod write;

pub use bash::{kill_background, preflight};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::cancel::Cancel;
use crate::config::{Bounds, TOOL_OUTPUT_CAP};
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
    pub async fn call(self, arguments: &str, cancel: &Cancel, bounds: &Bounds) -> Result<Outcome> {
        let raw = if arguments.trim().is_empty() {
            "{}"
        } else {
            arguments
        };
        let bound = bounds
            .confine
            .bounds_files()
            .then_some(bounds.root.as_path());
        let out: Outcome = match self {
            // `read` is never bounded: `bash` reads the whole filesystem in every mode, so a jail
            // here would only push the model through `cat`. `write` and `edit` are, because they
            // are otherwise the way around the sandbox's write policy.
            Tool::Read => read::call(parse(raw)?).await?.into(),
            Tool::Write => {
                let mut args: write::Args = parse(raw)?;
                if let Some(root) = bound {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                write::call(args).await?.into()
            }
            Tool::Edit => {
                let mut args: edit::Args = parse(raw)?;
                if let Some(root) = bound {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                edit::call(args).await?.into()
            }
            Tool::Bash => bash::call(parse(raw)?, cancel, bounds).await?,
        };
        Ok(Outcome {
            body: cap(out.body),
            note: out.note,
        })
    }
}

/// Resolve an existing path, or the nearest existing parent of a new path, before a tool opens it.
/// This follows symlinks and removes `..`, so the check is about the filesystem location.
///
/// The bound is the root alone, not the wider set of paths `bash` may write: nothing needs the
/// `write` tool to reach a build cache. Enforced in minima's own process, so unlike the kernel
/// policy it has a window between this check and the open.
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
    if let Some(name) = protected(root, &resolved) {
        bail!("path {raw:?} is protected: {name} is not writable through write or edit");
    }
    Ok(resolved)
}

/// The protected component of a resolved path, if it has one.
///
/// `.git` because a damaged object store loses history nothing can rebuild, and `.env` because a
/// replaced secret is not in the repository to restore. `.git` matches whole, leaving `.github`
/// and `.gitignore` alone. `.env` matches as a prefix, so `.env.production` is covered, minus the
/// names that hold no secret by convention: a template and `.envrc` are committed files, and
/// blocking them would refuse the edit that adding a config variable actually needs. Only
/// components below the root count: a root that itself sits under a `.git` directory is the
/// user's choice.
///
/// `bash` ignores this, on both platforms. Landlock grants access by union over the rules met
/// walking a path, so the rule granting the root cannot have a hole cut in it, and a boundary
/// that held only on macOS would be worse than none. See `docs/dev/root-sandbox.md`.
fn protected(root: &std::path::Path, resolved: &std::path::Path) -> Option<String> {
    const COMMITTED: [&str; 3] = ["example", "sample", "template"];
    let relative = resolved.strip_prefix(root).ok()?;
    relative.components().find_map(|component| {
        let name = component.as_os_str().to_str()?;
        let secret = name.starts_with(".env")
            && name != ".envrc"
            && !COMMITTED.iter().any(|kind| name.contains(kind));
        (name == ".git" || secret).then(|| name.to_string())
    })
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
    use crate::config::Confine;

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

    #[test]
    fn a_protected_path_under_the_root_is_refused_and_a_lookalike_is_not() {
        let dir = Scratch::new("protected");
        // `confine_path` resolves the path and not the root, as `Cli::resolve_root` does that once.
        let root = std::fs::canonicalize(dir.file("")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        for path in [".git", ".git/config", ".env", ".env.local", "sub/.env"] {
            let err = confine_path(&root, path).unwrap_err();
            assert!(err.to_string().contains("is protected"), "{path}: {err}");
        }
        for path in [
            ".github/ci.yml",
            ".gitignore",
            "env.sample",
            "src/environment.rs",
            ".env.example",
            ".env.local.template",
            ".envrc",
        ] {
            confine_path(&root, path).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
    }

    /// The guard bounds the two file tools. `read` is untouched, and so is `bash`.
    #[tokio::test]
    async fn a_protected_file_survives_write_and_stays_readable() {
        let dir = Scratch::new("protected-write");
        let root = std::fs::canonicalize(dir.file("")).unwrap();
        let path = dir.file(".env");
        std::fs::write(&path, "SECRET=1\n").unwrap();

        let err = Tool::Write
            .call(
                &serde_json::json!({ "path": path, "content": "x" }).to_string(),
                &Cancel::new(),
                &Bounds::new(Confine::Paths, root.clone()),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("is protected"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "SECRET=1\n");

        let out = Tool::Read
            .call(
                &serde_json::json!({ "path": path }).to_string(),
                &Cancel::new(),
                &Bounds::new(Confine::Paths, root.clone()),
            )
            .await
            .unwrap();
        assert!(out.body.contains("SECRET=1"), "{out:?}");
    }

    #[tokio::test]
    async fn confine_none_allows_a_write_outside_the_root() {
        let outside = Scratch::new("confine-none-outside");
        let path = outside.file("written.txt");

        Tool::Write
            .call(
                &serde_json::json!({ "path": path, "content": "x" }).to_string(),
                &Cancel::new(),
                &Bounds::new(Confine::None, std::path::PathBuf::from(outside.file(""))),
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }

    /// The mode decides two things, and the wiring between them is what is new: `paths` bounds
    /// the file tools and leaves `bash` alone, `fs` bounds both. `$HOME` is the one place outside
    /// the root that is neither the temp directory nor a build cache.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn only_fs_bounds_bash() {
        let dir = Scratch::new("confine-bash");
        let root = std::fs::canonicalize(dir.file("")).unwrap();
        let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
            return;
        };
        if !home.is_dir() {
            return;
        }
        let probe = home.join(format!(".minima-confine-{}", std::process::id()));
        let args =
            serde_json::json!({ "command": format!("touch '{}'", probe.display()) }).to_string();

        let out = Tool::Bash
            .call(
                &args,
                &Cancel::new(),
                &Bounds::new(Confine::Paths, root.clone()),
            )
            .await
            .unwrap();
        let reached = probe.exists();
        let _ = std::fs::remove_file(&probe);
        assert!(reached, "paths must not bound bash: {out:?}");

        let out = Tool::Bash
            .call(
                &args,
                &Cancel::new(),
                &Bounds::new(Confine::Fs, root.clone()),
            )
            .await
            .unwrap();
        let reached = probe.exists();
        let _ = std::fs::remove_file(&probe);
        assert!(!reached, "fs must bound bash: {out:?}");
    }

    #[tokio::test]
    async fn a_write_outside_the_root_is_refused() {
        let root = Scratch::new("write-boundary-root");
        let outside = Scratch::new("write-boundary-outside");
        let path = outside.file("kept.txt");
        std::fs::write(&path, "kept").unwrap();

        let err = Tool::Write
            .call(
                &serde_json::json!({ "path": path, "content": "x" }).to_string(),
                &Cancel::new(),
                &Bounds::new(Confine::Paths, std::path::PathBuf::from(root.file("."))),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside root"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "kept");
    }

    /// The sandbox bounds writes, not reads, and `bash` reads everything regardless.
    #[tokio::test]
    async fn a_read_outside_the_root_is_allowed() {
        let root = Scratch::new("read-boundary-root");
        let outside = Scratch::new("read-boundary-outside");
        let path = outside.file("notes.txt");
        std::fs::write(&path, "visible\n").unwrap();

        let out = Tool::Read
            .call(
                &serde_json::json!({ "path": path }).to_string(),
                &Cancel::new(),
                &Bounds::new(Confine::Paths, std::path::PathBuf::from(root.file("."))),
            )
            .await
            .unwrap();
        assert!(out.body.contains("visible"), "{out:?}");
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
