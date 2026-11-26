//! Four tools, dispatched by enum rather than `dyn Tool`.
//!
//! The freeze pays for itself here: with the set closed at four, an enum removes the trait
//! object, the `async-trait` dependency, and the registry. Adding a fifth tool is three lines.
//! `Builtin` hands the set to the agent loop in `minima-core`.

mod atomic;
mod bash;
mod edit;
#[cfg(feature = "codemode")]
pub mod js;
mod read;
mod write;

pub use bash::{kill_background, preflight};
pub use minima_core::tools::{CANCELLED, Outcome, Spec, cap};

use anyhow::{Context, Result, bail};
use futures_util::future::LocalBoxFuture;

use minima_core::tools::{or_cancelled, parse, schema_of};
use sanduk_sandbox::confine_path;

use crate::config::Bounds;
use minima_core::cancel::Cancel;

#[cfg(test)]
pub use minima_core::tools::Scratch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Read,
    Write,
    Edit,
    Bash,
    #[cfg(feature = "codemode")]
    Js,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Read, Tool::Write, Tool::Edit, Tool::Bash];

    pub fn name(self) -> &'static str {
        match self {
            Tool::Read => "read",
            Tool::Write => "write",
            Tool::Edit => "edit",
            Tool::Bash => "bash",
            #[cfg(feature = "codemode")]
            Tool::Js => "js",
        }
    }

    /// Only the tools this run advertises, so a call to any other is refused.
    pub fn from_name(name: &str) -> Option<Self> {
        active().into_iter().find(|t| t.name() == name)
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
            #[cfg(feature = "codemode")]
            Tool::Js => js::description(),
        }
    }

    fn parameters(self) -> serde_json::Value {
        match self {
            Tool::Read => schema_of::<read::Args>(),
            Tool::Write => schema_of::<write::Args>(),
            Tool::Edit => schema_of::<edit::Args>(),
            Tool::Bash => schema_of::<bash::Args>(),
            #[cfg(feature = "codemode")]
            Tool::Js => schema_of::<js::Args>(),
        }
    }

    pub fn spec(self) -> Spec {
        Spec {
            name: self.name().into(),
            description: self.description().into(),
            parameters: self.parameters(),
        }
    }

    /// `arguments` is the raw JSON string the model streamed, parsed here and nowhere else.
    pub async fn call(self, arguments: &str, cancel: &Cancel, bounds: &Bounds) -> Result<Outcome> {
        let raw = arguments;
        let bound = bounds.sandbox.then_some(bounds.root.as_path());
        let out: Outcome = match self {
            // `read` is never bounded: `bash` reads the whole filesystem in every mode, so a jail
            // here would only push the model through `cat`. `write` and `edit` are, because they
            // are otherwise the way around the sandbox's write policy.
            Tool::Read => or_cancelled(cancel, read::call(parse(raw)?)).await?,
            Tool::Write => {
                let mut args: write::Args = parse(raw)?;
                if let Some(root) = bound {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                or_cancelled(cancel, write::call(args)).await?
            }
            Tool::Edit => {
                let mut args: edit::Args = parse(raw)?;
                if let Some(root) = bound {
                    args.path = confine_path(root, &args.path)?.display().to_string();
                }
                or_cancelled(cancel, edit::call(args)).await?
            }
            Tool::Bash => bash::call(parse(raw)?, cancel, bounds).await?,
            #[cfg(feature = "codemode")]
            Tool::Js => Box::pin(js::call(parse(raw)?, cancel, bounds)).await?,
        };
        Ok(Outcome {
            body: cap(out.body),
            note: out.note,
        })
    }
}

/// The four tools under one run's bounds, as the agent loop calls them.
pub struct Builtin(pub Bounds);

impl minima_core::tools::Tools for Builtin {
    fn specs(&self) -> Vec<Spec> {
        active().into_iter().map(Tool::spec).collect()
    }

    fn call<'a>(
        &'a self,
        name: &'a str,
        arguments: &'a str,
        cancel: &'a Cancel,
    ) -> LocalBoxFuture<'a, Result<Outcome>> {
        Box::pin(async move {
            match Tool::from_name(name) {
                Some(tool) => tool.call(arguments, cancel, &self.0).await,
                None => bail!("no such tool: {name}"),
            }
        })
    }
}

/// The tools this run advertises. Only the `codemode` experiment varies the set.
fn active() -> Vec<Tool> {
    #[cfg(feature = "codemode")]
    return js::active();
    #[cfg(not(feature = "codemode"))]
    Tool::ALL.to_vec()
}

/// Opens a regular file for reading. `O_NONBLOCK` lets the open of a FIFO return without a writer,
/// so the type check runs instead of the call hanging; regular files ignore the flag.
async fn open_regular(path: &str) -> Result<tokio::fs::File> {
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .await
        .with_context(|| format!("reading {path}"))?;
    // A device has no end, and a directory's `EISDIR` from the first read says less than this.
    let meta = file
        .metadata()
        .await
        .with_context(|| format!("reading {path}"))?;
    if !meta.is_file() {
        bail!("{path} is not a regular file");
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use minima_core::provider::Dialect;

    /// A tool the model is never told about cannot be called, and a mock cannot catch that.
    /// Each dialect reads the name and schema from a different place.
    #[test]
    fn every_tool_advertises_an_object_schema_in_every_dialect() {
        for tool in Tool::ALL {
            let chat = tool.spec().wire(Dialect::Chat);
            assert_eq!(chat["function"]["name"], tool.name());
            assert_eq!(chat["function"]["parameters"]["type"], "object");

            let responses = tool.spec().wire(Dialect::Responses);
            assert_eq!(responses["name"], tool.name());
            assert_eq!(responses["parameters"]["type"], "object");

            let messages = tool.spec().wire(Dialect::Messages);
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
        use minima_core::tools::Tools;
        let builtin = Builtin(Bounds::new(false, std::env::temp_dir()));
        assert_eq!(builtin.specs().len(), 4);
    }

    #[test]
    fn names_round_trip() {
        for tool in Tool::ALL {
            assert_eq!(Tool::from_name(tool.name()), Some(tool));
        }
        assert_eq!(Tool::from_name("grep"), None);
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
                &Bounds::new(true, root.clone()),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("is protected"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "SECRET=1\n");

        let out = Tool::Read
            .call(
                &serde_json::json!({ "path": path }).to_string(),
                &Cancel::new(),
                &Bounds::new(true, root.clone()),
            )
            .await
            .unwrap();
        assert!(out.body.contains("SECRET=1"), "{out:?}");
    }

    #[tokio::test]
    async fn without_the_sandbox_a_write_outside_the_root_lands() {
        let outside = Scratch::new("sandbox-off-outside");
        let path = outside.file("written.txt");

        Tool::Write
            .call(
                &serde_json::json!({ "path": path, "content": "x" }).to_string(),
                &Cancel::new(),
                &Bounds::new(false, std::path::PathBuf::from(outside.file(""))),
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }

    /// Off, `bash` reaches outside the root; on, it does not. `$HOME` is the one place outside
    /// the root that is neither the temp directory nor a build cache.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn only_the_sandbox_bounds_bash() {
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
            .call(&args, &Cancel::new(), &Bounds::new(false, root.clone()))
            .await
            .unwrap();
        let reached = probe.exists();
        let _ = std::fs::remove_file(&probe);
        assert!(
            reached,
            "bash must be unbounded without the sandbox: {out:?}"
        );

        let out = Tool::Bash
            .call(&args, &Cancel::new(), &Bounds::new(true, root.clone()))
            .await
            .unwrap();
        let reached = probe.exists();
        let _ = std::fs::remove_file(&probe);
        assert!(!reached, "the sandbox must bound bash: {out:?}");
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
                &Bounds::new(true, std::path::PathBuf::from(root.file("."))),
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
                &Bounds::new(true, std::path::PathBuf::from(root.file("."))),
            )
            .await
            .unwrap();
        assert!(out.body.contains("visible"), "{out:?}");
    }

    fn fifo(dir: &Scratch) -> String {
        let path = dir.file("pipe");
        let c = std::ffi::CString::new(path.clone()).unwrap();
        // SAFETY: `c` is a valid NUL-terminated path for the duration of the call.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        path
    }

    /// A FIFO with no writer blocks `open(2)`, so the type check must not wait for the open.
    #[tokio::test]
    async fn read_and_edit_refuse_a_fifo_without_blocking() {
        let dir = Scratch::new("fifo");
        let path = fifo(&dir);
        let bounds = Bounds::new(false, std::env::temp_dir());
        for (tool, args) in [
            (Tool::Read, serde_json::json!({ "path": path })),
            (
                Tool::Edit,
                serde_json::json!({ "path": path, "old": "a", "new": "b" }),
            ),
        ] {
            let (args, cancel) = (args.to_string(), Cancel::new());
            let call = tool.call(&args, &cancel, &bounds);
            let err = tokio::time::timeout(std::time::Duration::from_secs(5), call)
                .await
                .expect("the call returned")
                .unwrap_err();
            assert!(err.to_string().contains("not a regular file"), "{err}");
        }
    }

    /// The transcript already holds the arguments; a failed 30 KiB `write` must not repeat them.
    #[tokio::test]
    async fn invalid_arguments_are_not_echoed_back() {
        let raw = format!("{{\"path\": \"x\", \"content\": \"{}", "y".repeat(1000));
        let err = Tool::Write
            .call(
                &raw,
                &Cancel::new(),
                &Bounds::new(false, std::env::temp_dir()),
            )
            .await
            .unwrap_err();
        let text = format!("{err:#}");
        assert!(text.starts_with("tool arguments were not valid"), "{text}");
        assert!(!text.contains("yyyy"), "{text}");
    }
}
