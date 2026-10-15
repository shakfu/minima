//! Experiment: codemode. Model-written JavaScript runs in QuickJS and calls the other tools.
//!
//! `MINIMA_CODEMODE` picks the variant under test; see `docs/dev/codemode.md`. Each call gets a
//! fresh runtime on its own thread, so tool calls block on the tokio handle without stalling a
//! worker, and nothing persists between calls.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use rquickjs::{CatchResultExt, CaughtError, Context, Ctx, Exception, Function, Runtime, Value};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::runtime::Handle;

use super::{CANCELLED, Outcome, Tool};
use crate::cancel::Cancel;
use crate::config::Bounds;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_TIMEOUT: Duration = Duration::from_secs(600);
const MEMORY: usize = 64 * 1024 * 1024;
const STACK: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The four tools, no aggregation line: the baseline.
    None,
    /// The four tools and the `bash` aggregation line: the default build.
    Line,
    /// The four tools plus `js`; scripts reach `read`, `write`, `edit`.
    Js,
    /// `read`, `write`, `edit` and `js`; no `bash` anywhere.
    JsOnly,
    /// The four tools plus `js`; scripts also reach `bash`, as in Pi.
    JsBash,
}

static VARIANT: OnceLock<Variant> = OnceLock::new();

/// Read once at startup, so a misspelt variant fails the run instead of measuring the default.
pub fn init_variant() -> Result<()> {
    let v = match std::env::var("MINIMA_CODEMODE").as_deref() {
        Err(_) | Ok("line") => Variant::Line,
        Ok("none") => Variant::None,
        Ok("js") => Variant::Js,
        Ok("js-only") => Variant::JsOnly,
        Ok("js-bash") => Variant::JsBash,
        Ok(other) => bail!("MINIMA_CODEMODE={other}: expected none, line, js, js-only or js-bash"),
    };
    let _ = VARIANT.set(v);
    Ok(())
}

pub fn variant() -> Variant {
    VARIANT.get().copied().unwrap_or(Variant::Line)
}

pub fn active() -> Vec<Tool> {
    match variant() {
        Variant::None | Variant::Line => Tool::ALL.to_vec(),
        Variant::Js | Variant::JsBash => {
            vec![Tool::Read, Tool::Write, Tool::Edit, Tool::Bash, Tool::Js]
        }
        Variant::JsOnly => vec![Tool::Read, Tool::Write, Tool::Edit, Tool::Js],
    }
}

pub fn description() -> &'static str {
    if variant() == Variant::JsBash {
        "Run JavaScript in QuickJS (ES2023, no filesystem, network or timers of its own). Tool \
         calls are synchronous: `tools.read(path)`, `tools.write({path, \
         content})`, `tools.edit({path, old, new, replace_all})`, `tools.bash({command, \
         timeout_ms})` return the tool's text or throw. `tools.ls(dir)` returns entry names, \
         directories ending in `/`. `tools.all([[name, args], ...])` runs calls concurrently and \
         returns their texts. Returns `console.log` output, then the last expression's value. \
         `write` and `edit` are confined to the working directory. `read` returns the whole file as text."
    } else {
        "Run JavaScript in QuickJS (ES2023, no filesystem, network or timers of its own). Tool \
         calls are synchronous: `tools.read(path)`, `tools.write({path, \
         content})`, `tools.edit({path, old, new, replace_all})` return the tool's text or \
         throw. `tools.ls(dir)` returns entry names, directories ending in `/`. \
         `tools.all([[name, args], ...])` runs calls concurrently and returns their texts. \
         Returns `console.log` output, then the last expression's value. `write` and `edit` are \
         confined to the working directory. `read` returns the whole file as text."
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// JavaScript to run. The value of the last expression is returned.
    pub code: String,
    /// Timeout in milliseconds. Defaults to 120000, capped at 600000.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

pub async fn call(args: Args, cancel: &Cancel, bounds: &Bounds) -> Result<Outcome> {
    let limit = args
        .timeout_ms
        .map_or(DEFAULT_TIMEOUT, Duration::from_millis)
        .min(MAX_TIMEOUT);
    let job = Job {
        handle: Handle::current(),
        cancel: cancel.clone(),
        bounds: bounds.clone(),
        bash: variant() == Variant::JsBash,
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(job.run(&args.code, limit));
    });
    rx.await
        .map_err(|_| anyhow!("the script thread panicked"))?
}

#[derive(Clone)]
struct Job {
    handle: Handle,
    cancel: Cancel,
    bounds: Bounds,
    bash: bool,
}

const PRELUDE: &str = r#"
(() => {
  const call = (name, args) => __call(name, JSON.stringify(args ?? {}));
  const shorthand = { read: "path", bash: "command" };
  const wrap = name => a => call(name, typeof a === "string" && shorthand[name] ? { [shorthand[name]]: a } : a);
  const tools = { ls: (path = ".") => JSON.parse(call("ls", { path })) };
  for (const name of __names) tools[name] = wrap(name);
  tools.all = calls => JSON.parse(__all(JSON.stringify(calls.map(([n, a]) =>
    [n, typeof a === "string" && shorthand[n] ? { [shorthand[n]]: a } : a ?? {}]))));
  globalThis.tools = tools;
  const show = x => typeof x === "string" ? x : JSON.stringify(x);
  globalThis.console = { log: (...xs) => __log(xs.map(show).join(" ")) };
  globalThis.console.error = globalThis.console.log;
  globalThis.__show = v => v === undefined ? "" : typeof v === "string" ? v : JSON.stringify(v, null, 2);
})();
"#;

impl Job {
    fn run(self, code: &str, limit: Duration) -> Result<Outcome> {
        let rt = Runtime::new().map_err(|e| anyhow!("{e}"))?;
        rt.set_memory_limit(MEMORY);
        rt.set_max_stack_size(STACK);
        let deadline = Instant::now() + limit;
        let watch = self.cancel.clone();
        rt.set_interrupt_handler(Some(Box::new(move || {
            watch.is_cancelled() || Instant::now() > deadline
        })));
        let context = Context::full(&rt).map_err(|e| anyhow!("{e}"))?;
        let logs = std::rc::Rc::new(std::cell::RefCell::new(String::new()));

        let result = context.with(|ctx| -> Result<String, String> {
            self.install(&ctx, &logs).map_err(|e| e.to_string())?;
            let value: Value = ctx.eval(code).catch(&ctx).map_err(describe)?;
            // An async wrapper resolves once its jobs run; tool calls inside it are synchronous.
            let value = match value.as_promise() {
                Some(p) => p.finish::<Value>().catch(&ctx).map_err(describe)?,
                None => value,
            };
            let show: Function = ctx.globals().get("__show").map_err(|e| e.to_string())?;
            show.call((value,)).catch(&ctx).map_err(describe)
        });

        if self.cancel.is_cancelled() {
            return Ok(CANCELLED.to_string().into());
        }
        let mut body = std::mem::take(&mut *logs.borrow_mut());
        match result {
            Ok(value) => {
                if !value.is_empty() {
                    if !body.is_empty() && !body.ends_with('\n') {
                        body.push('\n');
                    }
                    body.push_str(&value);
                }
                if body.is_empty() {
                    body.push_str("(no output)");
                }
                Ok(body.into())
            }
            Err(_) if Instant::now() > deadline => {
                bail!("{body}script timed out after {} ms", limit.as_millis())
            }
            Err(e) => bail!("{body}{e}"),
        }
    }

    fn install<'js>(
        &self,
        ctx: &Ctx<'js>,
        logs: &std::rc::Rc<std::cell::RefCell<String>>,
    ) -> rquickjs::Result<()> {
        let globals = ctx.globals();
        let mut names = vec!["read", "write", "edit"];
        if self.bash {
            names.push("bash");
        }
        globals.set("__names", names)?;

        let logs = logs.clone();
        globals.set(
            "__log",
            Function::new(ctx.clone(), move |line: String| {
                let mut logs = logs.borrow_mut();
                logs.push_str(&line);
                logs.push('\n');
            })?,
        )?;

        let job = self.clone();
        globals.set(
            "__call",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, name: String, args: String| -> rquickjs::Result<String> {
                    job.handle
                        .block_on(job.dispatch(&name, &args))
                        .map_err(|e| Exception::throw_message(&ctx, &format!("{e:#}")))
                },
            )?,
        )?;

        let job = self.clone();
        globals.set(
            "__all",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, calls: String| -> rquickjs::Result<String> {
                    let calls: Vec<(String, serde_json::Value)> = serde_json::from_str(&calls)
                        .map_err(|e| Exception::throw_message(&ctx, &format!("tools.all: {e}")))?;
                    let calls: Vec<(String, String)> =
                        calls.into_iter().map(|(n, a)| (n, a.to_string())).collect();
                    let results = job.handle.block_on(futures_util::future::join_all(
                        calls.iter().map(|(name, args)| job.dispatch(name, args)),
                    ));
                    let texts: Vec<String> = results
                        .into_iter()
                        .map(|r| r.unwrap_or_else(|e| format!("error: {e:#}")))
                        .collect();
                    Ok(serde_json::to_string(&texts).unwrap_or_default())
                },
            )?,
        )?;
        let _: Value = ctx.eval(PRELUDE)?;
        Ok(())
    }

    async fn dispatch(&self, name: &str, args: &str) -> Result<String> {
        let tool = match name {
            "read" => return read(args).await,
            "write" => Tool::Write,
            "edit" => Tool::Edit,
            "bash" if self.bash => Tool::Bash,
            "ls" => return ls(args).await,
            _ => bail!("no such tool: {name}"),
        };
        // Scripts never write outside the root, sandbox or not: that is the variant under test.
        let bounds = match tool {
            Tool::Write | Tool::Edit => Bounds {
                sandbox: true,
                ..self.bounds.clone()
            },
            _ => self.bounds.clone(),
        };
        Ok(tool.call(args, &self.cancel, &bounds).await?.body)
    }
}

/// Larger than any source file, and well inside `MEMORY`.
const READ_CAP: u64 = 8 * 1024 * 1024;

#[derive(Deserialize)]
struct ReadArgs {
    path: String,
}

/// The whole file, not the `read` tool's numbered and capped view: a script that counts lines in
/// a file past the tool cap would otherwise count a prefix and report it without error.
async fn read(args: &str) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let ReadArgs { path } = serde_json::from_str(args)?;
    let file = super::open_regular(&path).await?;
    let mut bytes = Vec::new();
    file.take(READ_CAP + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > READ_CAP {
        bail!("{path} is over {} MiB", READ_CAP / 1024 / 1024);
    }
    String::from_utf8(bytes).map_err(|_| anyhow!("{path} is not UTF-8 text"))
}

#[derive(Deserialize)]
struct LsArgs {
    path: String,
}

async fn ls(args: &str) -> Result<String> {
    let LsArgs { path } = serde_json::from_str(args)?;
    let mut dir = tokio::fs::read_dir(&path)
        .await
        .map_err(|e| anyhow!("listing {path}: {e}"))?;
    let mut names = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let mut name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().await?.is_dir() {
            name.push('/');
        }
        names.push(name);
    }
    names.sort();
    Ok(serde_json::to_string(&names)?)
}

/// The exception's message and stack, or the engine error.
fn describe(e: CaughtError<'_>) -> String {
    match e {
        CaughtError::Exception(ex) => {
            let name: String = ex.get("name").unwrap_or_else(|_| "Error".into());
            let message = format!("{name}: {}", ex.message().unwrap_or_default());
            match ex.stack() {
                Some(stack) if !stack.is_empty() => format!("{message}\n{stack}"),
                _ => message,
            }
        }
        CaughtError::Value(v) => format!("thrown: {v:?}"),
        CaughtError::Error(e) => e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    async fn run(code: &str, bounds: &Bounds) -> Result<Outcome> {
        let args = Args {
            code: code.into(),
            timeout_ms: Some(2_000),
        };
        call(args, &Cancel::new(), bounds).await
    }

    fn bounds(dir: &Scratch) -> Bounds {
        Bounds::new(false, std::fs::canonicalize(dir.file("")).unwrap())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_script_reads_files_and_returns_only_its_result() {
        let dir = Scratch::new("js-read");
        std::fs::write(dir.file("a.txt"), "x\ny\nz\n").unwrap();
        std::fs::write(dir.file("b.txt"), "x\n").unwrap();
        let code = format!(
            "const d = {:?}; tools.ls(d).map(f => [f, tools.read(d + '/' + f).split('\\n').filter(Boolean).length])",
            dir.file("")
        );
        let out = run(&code, &bounds(&dir)).await.unwrap();
        let counts: serde_json::Value = serde_json::from_str(&out.body).unwrap();
        assert_eq!(counts, serde_json::json!([["a.txt", 3], ["b.txt", 1]]));
    }

    /// The `read` tool stops at the output cap; a script must see the whole file.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_script_reads_past_the_tool_output_cap() {
        let dir = Scratch::new("js-big");
        let lines = crate::config::TOOL_OUTPUT_CAP / 10;
        std::fs::write(dir.file("big.txt"), "123456789\n".repeat(lines)).unwrap();
        let code = format!(
            "tools.read({:?}).split('\\n').length - 1",
            dir.file("big.txt")
        );
        let out = run(&code, &bounds(&dir)).await.unwrap();
        assert_eq!(out.body, lines.to_string());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn logs_come_before_the_value() {
        let dir = Scratch::new("js-log");
        let out = run("console.log('a', {b: 1}); 2 + 2", &bounds(&dir))
            .await
            .unwrap();
        assert_eq!(out.body, "a {\"b\":1}\n4");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_async_wrapper_is_awaited() {
        let dir = Scratch::new("js-async");
        let out = run("(async () => 'done')()", &bounds(&dir)).await.unwrap();
        assert_eq!(out.body, "done");
    }

    /// Without `--sandbox`, the direct `write` tool is unbounded; a script's is not.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_script_cannot_write_outside_the_root_without_the_sandbox() {
        let root = Scratch::new("js-root");
        let outside = Scratch::new("js-outside");
        let path = outside.file("x.txt");
        let code = format!("tools.write({{path: {path:?}, content: 'x'}})");
        let err = run(&code, &bounds(&root)).await.unwrap_err();
        assert!(err.to_string().contains("outside root"), "{err}");
        assert!(!std::path::Path::new(&path).exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bash_is_absent_unless_the_variant_grants_it() {
        let dir = Scratch::new("js-bash");
        let err = run("tools.bash('true')", &bounds(&dir)).await.unwrap_err();
        assert!(err.to_string().contains("not a function"), "{err}");
        let err = run("__call('bash', '{\"command\":\"true\"}')", &bounds(&dir))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no such tool: bash"), "{err}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_infinite_loop_times_out() {
        let dir = Scratch::new("js-loop");
        let args = Args {
            code: "while (true) {}".into(),
            timeout_ms: Some(200),
        };
        let err = call(args, &Cancel::new(), &bounds(&dir)).await.unwrap_err();
        assert!(err.to_string().contains("timed out after 200 ms"), "{err}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_cancel_interrupts_a_running_script() {
        let dir = Scratch::new("js-cancel");
        let cancel = Cancel::new();
        let later = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            later.cancel();
        });
        let args = Args {
            code: "while (true) {}".into(),
            timeout_ms: Some(10_000),
        };
        let out = call(args, &cancel, &bounds(&dir)).await.unwrap();
        assert_eq!(out.body, CANCELLED);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn memory_is_bounded() {
        let dir = Scratch::new("js-memory");
        let err = run(
            "let a = []; while (true) a.push(new Array(1e5).fill('x'));",
            &bounds(&dir),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("out of memory"), "{err}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn all_runs_calls_and_reports_each_failure_in_place() {
        let dir = Scratch::new("js-all");
        std::fs::write(dir.file("a.txt"), "hello\n").unwrap();
        let code = format!(
            "tools.all([['read', {:?}], ['read', {:?}]])",
            dir.file("a.txt"),
            dir.file("missing.txt")
        );
        let out = run(&code, &bounds(&dir)).await.unwrap();
        let texts: Vec<String> = serde_json::from_str(&out.body).unwrap();
        assert!(texts[0].contains("hello"), "{texts:?}");
        assert!(texts[1].starts_with("error:"), "{texts:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_syntax_error_is_reported() {
        let dir = Scratch::new("js-syntax");
        let err = run("let = ;", &bounds(&dir)).await.unwrap_err();
        assert!(err.to_string().starts_with("SyntaxError: "), "{err}");
    }
}
