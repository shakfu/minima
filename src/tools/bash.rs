//! The command runs in its own process group. A timeout or cancel kills the whole group. A clean
//! exit leaves background jobs running, so a server can serve the next call, and minima kills
//! them when it exits. A job that writes to stdout or stderr after its call returns gets SIGPIPE.

use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::task::JoinHandle;

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

use super::Outcome;
use crate::cancel::Cancel;
use crate::config::{Bounds, TOOL_OUTPUT_CAP};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_TIMEOUT: Duration = Duration::from_secs(600);
/// How long to keep reading after bash exits. EOF arrives at once unless a background job holds
/// the pipe, and then this bounds the wait.
const DRAIN_GRACE: Duration = Duration::from_millis(100);
/// Bytes kept per stream while the command runs. `tools::cap` also trims the finished body, but
/// only once the whole of it is in memory: a command that writes without stopping would be gone
/// by then. Matching the context cap means a command that stays under it is captured whole.
const CAPTURE_CAP: usize = TOOL_OUTPUT_CAP;
const LEFT_RUNNING: &str = "background jobs still running; minima kills them when it exits";
const LOGIN_SHELL: &str = "not run: the command already runs under `bash -c`, and a login shell \
reorders PATH, so programs can resolve differently; pass the inner command without the wrapper";

/// Process groups that may still have members: running calls, and finished calls that left jobs.
///
/// An id is safe to signal while its group has a member, because POSIX does not reuse it until
/// then. A group that empties on its own frees the id, so the kill at exit could reach a new group
/// with the same id. That needs the pid counter to wrap within one session.
static GROUPS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn groups() -> std::sync::MutexGuard<'static, Vec<u32>> {
    GROUPS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Kills every group a call left running. Called as minima exits.
pub fn kill_background() {
    for id in std::mem::take(&mut *groups()) {
        signal_group(id, SIGKILL);
    }
}

#[cfg(unix)]
const SIGKILL: i32 = libc::SIGKILL;
#[cfg(not(unix))]
const SIGKILL: i32 = 9;

/// True when the group still had a member to receive the signal.
fn signal_group(id: u32, signal: i32) -> bool {
    #[cfg(unix)]
    if let Ok(pgid) = libc::pid_t::try_from(id) {
        // SAFETY: killpg takes no pointers. A group that is already gone returns ESRCH.
        return unsafe { libc::killpg(pgid, signal) } == 0;
    }
    let _ = (id, signal);
    false
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Shell command to run.
    pub command: String,
    /// Timeout in milliseconds. Defaults to 120000, capped at 600000.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

pub async fn call(args: Args, cancel: &Cancel, bounds: &Bounds) -> Result<Outcome> {
    if login_shell(&args.command) {
        bail!(LOGIN_SHELL);
    }
    let limit = args
        .timeout_ms
        .map_or(DEFAULT_TIMEOUT, Duration::from_millis)
        .min(MAX_TIMEOUT);

    let mut command = if bounds.confine.sandboxes_bash() {
        sandbox_command(bounds, &args.command)?
    } else {
        plain_command(&args.command)
    };
    command
        .current_dir(&bounds.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .with_context(|| format!("spawning: {}", args.command))?;

    // Declared after `child`, so it drops first and signals the group before bash is reaped.
    let group = Group::register(child.id());
    let stdout = Drain::start(child.stdout.take());
    let stderr = Drain::start(child.stderr.take());

    // Biased: a finished wait is taken before the cancel, so the group is never signalled after
    // bash has been reaped and its id could belong to another process.
    let finished = tokio::select! {
        biased;
        result = tokio::time::timeout(limit, child.wait()) => result,
        () = cancel.cancelled() => return Ok(super::CANCELLED.to_string().into()),
    };

    let Ok(status) = finished else {
        drop(group);
        let note = format!("timed out after {}ms", limit.as_millis());
        let mut body = output(stdout, stderr).await.0;
        body.push_str(&format!("\n({note})"));
        return Ok(Outcome {
            body,
            note: Some(note),
        });
    };
    let left_running = group.release();
    let status = status.context("waiting for the command")?;
    let (mut body, stderr) = output(stdout, stderr).await;

    // A non-zero exit is information for the model, not a tool failure. The note is what the user
    // sees, so it leads with the reason rather than the whole output.
    //
    // Exit status alone is not enough. In a pipeline the status belongs to the last stage, so
    // `find -printf ... | wc -c` exits 0 while find's error goes to stderr unseen. Anything on
    // stderr is therefore worth a line, whatever the exit code says.
    let reason = first_line(&stderr);
    let mut note = match (status.code(), reason) {
        (Some(0), None) => None,
        (Some(0), Some(line)) => Some(format!("stderr: {line}")),
        (Some(code), Some(line)) => Some(format!("exit {code}: {line}")),
        (Some(code), None) => Some(format!("exit {code}")),
        (None, _) => Some("killed by a signal".to_string()),
    };
    if bounds.confine.sandboxes_bash() && looks_denied(&stderr) {
        note = Some(match note {
            Some(note) => format!("{note}; {DENIED}"),
            None => DENIED.to_string(),
        });
    }
    // Said once, to the model and the user, so neither loses track of a job it started.
    if left_running {
        note = Some(match note {
            Some(note) => format!("{note}; {LEFT_RUNNING}"),
            None => LEFT_RUNNING.to_string(),
        });
    }

    if let Some(note) = &note {
        body.push_str(&format!("\n({note})"));
    }
    Ok(Outcome { body, note })
}

fn plain_command(command: &str) -> Command {
    let mut process = Command::new("bash");
    process.arg("-c").arg(command);
    process
}

/// Paths outside the root that stay writable. A shell needs the temp directory and `/dev/null`;
/// a build needs the ecosystem caches. Measured 2026-09-19: an offline `cargo build` opens
/// `$CARGO_HOME/.package-cache` with `O_RDWR|O_CREAT` on every run, so a policy without the
/// caches denies the build, not just the dependency fetch. A lost cache costs a re-download
/// rather than work, which is why they sit on the permissive side of the line.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn writable_outside_root() -> Vec<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let named = |var: &str, under_home: &str| {
        std::env::var_os(var)
            .map(std::path::PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(under_home)))
    };
    [
        // Reads TMPDIR, which on macOS is a per-user path under /var/folders rather than /tmp.
        Some(std::env::temp_dir()),
        Some(std::path::PathBuf::from("/dev/null")),
        named("CARGO_HOME", ".cargo"),
        named("XDG_CACHE_HOME", ".cache"),
        named("GOPATH", "go"),
        home.as_ref().map(|h| h.join(".npm")),
        platform_cache(home.as_ref()),
    ]
    .into_iter()
    .flatten()
    // Canonical, because Seatbelt matches a profile against the resolved path: on macOS `/tmp` is
    // a symlink to `/private/tmp`, and `$TMPDIR` carries a trailing slash that `subpath` will not
    // match. Dropping what does not resolve also drops what does not exist.
    .filter_map(|path| std::fs::canonicalize(path).ok())
    .collect()
}

/// macOS keeps user caches under `~/Library/Caches`, not `$XDG_CACHE_HOME`, so the entry above is
/// the Linux half of one rule. Go's build cache is `~/Library/Caches/go-build` on a Mac, and
/// measured 2026-09-20: `go build` fails outright when that directory does not yet exist and
/// cannot be created, and silently stops caching when it exists but is not writable.
#[cfg(target_os = "macos")]
fn platform_cache(home: Option<&std::path::PathBuf>) -> Option<std::path::PathBuf> {
    home.map(|h| h.join("Library/Caches"))
}

#[cfg(target_os = "linux")]
fn platform_cache(_home: Option<&std::path::PathBuf>) -> Option<std::path::PathBuf> {
    None
}

/// What a denied open looks like from inside the command. Seatbelt returns `EPERM` and Landlock
/// `EACCES`, and the program prints its own message, which never names the policy: a model that
/// reads "Operation not permitted" retries the command or reaches for `sudo`. Matching the text
/// is a heuristic -- an ordinary permission error gets the line too, and a translated system gets
/// nothing -- and one extra sentence costs less than a retry loop.
const DENIED: &str = "if a write was denied: --confine fs permits writes under the root, $TMPDIR, /dev/null and the build caches; another directory needs --writable, or a store inside the root";

fn looks_denied(stderr: &str) -> bool {
    stderr.contains("Operation not permitted") || stderr.contains("Permission denied")
}

/// Reads are allowed everywhere; writes only under the root and `writable_outside_root`. The
/// policy bounds what a command can destroy, not what it can see. Headers, toolchains and
/// dependency sources sit outside the root, and the network is open either way, so denying reads
/// would cost capability without closing exfiltration.
#[cfg(target_os = "linux")]
fn sandbox_command(bounds: &Bounds, command: &str) -> Result<Command> {
    use landlock::{
        ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr, RulesetStatus, path_beneath_rules,
    };

    // V3 is the floor. V1 denies every rename across directories, which would break `mv` inside
    // the root, and without V3's `Truncate` a read-only grant still permits truncating any file
    // on the system. `IoctlDev` arrives in V5 and is not handled, so ioctls on device files the
    // command can open stay unrestricted.
    let abi = ABI::V3;
    let write = AccessFs::from_all(abi);
    let ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(write)
        .context("configuring the Linux filesystem sandbox")?
        .create()
        .context("creating the Linux filesystem sandbox")?
        .add_rule(PathBeneath::new(
            PathFd::new("/").context("opening /")?,
            AccessFs::from_read(abi),
        ))
        .context("allowing reads")?
        .add_rule(PathBeneath::new(
            PathFd::new(&bounds.root).context("opening the sandbox root")?,
            write,
        ))
        .context("allowing the sandbox root")?
        // Drops a path that does not open, and masks the directory-only rights that would be
        // rejected on a file, which `/dev/null` is. A `--writable` path is already resolved, and
        // is added here rather than earlier so a missing one is reported by its own flag.
        .add_rules(path_beneath_rules(writable_outside_root(), write))
        .context("allowing the writable paths outside the root")?
        .add_rules(path_beneath_rules(bounds.writable.clone(), write))
        .context("allowing the paths --writable named")?;

    let mut ruleset = Some(ruleset);
    let mut process = Command::new("bash");
    process.arg("-c").arg(command);
    // SAFETY: the closure only consumes the prebuilt ruleset and performs syscalls in the child.
    unsafe {
        process.as_std_mut().pre_exec(move || {
            let status = ruleset
                .take()
                .ok_or_else(|| std::io::Error::other("sandbox pre-exec ran twice"))?
                .restrict_self()
                .map_err(std::io::Error::other)?;
            if status.ruleset != RulesetStatus::FullyEnforced {
                return Err(std::io::Error::other(
                    "Linux filesystem sandbox was not fully enforced",
                ));
            }
            Ok(())
        });
    }
    Ok(process)
}

/// Absolute, not `sandbox-exec` on `PATH`: a shim earlier in the search path would exec its
/// argument unconfined, and `preflight` would take its exit 0 as a working sandbox.
#[cfg(target_os = "macos")]
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// The macOS policy in SBPL. Allow-default with writes denied, rather than deny-default: a
/// deny-default profile has to name every path a toolchain reads, and a missing one fails the
/// command outright. `.github/workflows/ci.yml` runs the suite on macOS, so the profile is
/// tested, but an allow-default profile is the shape whose mistakes are recoverable.
#[cfg(target_os = "macos")]
fn sandbox_command(bounds: &Bounds, command: &str) -> Result<Command> {
    let mut profile =
        String::from("(version 1) (allow default) (deny file-write*) (allow file-write*");
    let named = std::iter::once(bounds.root.clone())
        .chain(writable_outside_root())
        .chain(bounds.writable.iter().cloned());
    for path in named {
        let form = if path.is_dir() { "subpath" } else { "literal" };
        // The backslash is replaced first, or it would escape the quote that follows it.
        let quoted = path
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        profile.push_str(&format!(" ({form} \"{quoted}\")"));
    }
    profile.push(')');

    let mut process = Command::new(SANDBOX_EXEC);
    process.args(["-p", &profile, "bash", "-c", command]);
    Ok(process)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn sandbox_command(_bounds: &Bounds, _command: &str) -> Result<Command> {
    bail!("this platform has no filesystem sandbox; --confine paths runs without one")
}

/// One confined command before the agent starts. A kernel without Landlock, or a macOS without
/// `sandbox-exec`, fails here rather than on whichever tool call the model makes first.
pub async fn preflight(bounds: &Bounds) -> Result<()> {
    let args = Args {
        command: "exit 0".into(),
        timeout_ms: Some(10_000),
    };
    call(args, &Cancel::new(), bounds).await.context(
        "the filesystem sandbox could not be installed; --confine paths runs without it",
    )?;
    Ok(())
}

/// Stdout followed by stderr as one body, plus stderr alone for the note.
async fn output(stdout: Drain, stderr: Drain) -> (String, String) {
    let (stdout, stderr) = tokio::join!(stdout.finish(), stderr.finish());
    let stderr = stderr.render();
    let mut body = stdout.render();
    if !body.is_empty() && !body.ends_with('\n') && !stderr.is_empty() {
        body.push('\n');
    }
    body.push_str(&stderr);
    if body.is_empty() {
        body.push_str("(no output)");
    }
    (body, stderr)
}

/// One call's process group, registered in `GROUPS`. SIGKILL to the whole group on drop, unless
/// released.
struct Group(Option<u32>);

impl Group {
    fn register(id: Option<u32>) -> Self {
        if let Some(id) = id {
            groups().push(id);
        }
        Self(id)
    }

    /// For a clean exit. True when background jobs are still running; the group then stays
    /// registered until minima exits.
    fn release(mut self) -> bool {
        let Some(id) = self.0.take() else {
            return false;
        };
        // Signal 0 checks for members without sending anything.
        let running = signal_group(id, 0);
        if !running {
            forget(id);
        }
        running
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        if let Some(id) = self.0 {
            signal_group(id, SIGKILL);
            forget(id);
        }
    }
}

fn forget(id: u32) {
    groups().retain(|&g| g != id);
}

/// The head and tail of a stream within a fixed budget, and how many bytes went through it.
/// Everything between the two ends is counted and dropped as it arrives, so `yes` costs
/// `CAPTURE_CAP` bytes rather than a killed process.
#[derive(Default)]
struct Capture {
    head: Vec<u8>,
    tail: VecDeque<u8>,
    total: usize,
}

impl Capture {
    fn push(&mut self, bytes: &[u8]) {
        self.total += bytes.len();
        let half = CAPTURE_CAP / 2;
        let room = half - self.head.len();
        let take = room.min(bytes.len());
        self.head.extend_from_slice(&bytes[..take]);
        self.tail.extend(&bytes[take..]);
        while self.tail.len() > half {
            self.tail.pop_front();
        }
    }

    /// Same shape as `tools::cap`, which trims the body again once stderr is appended to it.
    fn render(self) -> String {
        let dropped = self.total - self.head.len() - self.tail.len();
        let tail: Vec<u8> = self.tail.into();
        let (head, tail) = (
            String::from_utf8_lossy(&self.head),
            String::from_utf8_lossy(&tail),
        );
        if dropped == 0 {
            return format!("{head}{tail}");
        }
        format!("{head}\n... {dropped} bytes elided ...\n{tail}")
    }
}

/// Reads a pipe in the background into a capture that can be taken before EOF.
struct Drain {
    buf: Arc<Mutex<Capture>>,
    task: JoinHandle<()>,
}

impl Drain {
    fn start(pipe: Option<impl AsyncRead + Unpin + Send + 'static>) -> Self {
        let buf = Arc::new(Mutex::new(Capture::default()));
        let task = tokio::spawn({
            let buf = Arc::clone(&buf);
            async move {
                let Some(mut pipe) = pipe else { return };
                let mut chunk = [0u8; 8192];
                while let Ok(n @ 1..) = pipe.read(&mut chunk).await {
                    buf.lock().expect("drain buffer").push(&chunk[..n]);
                }
            }
        });
        Self { buf, task }
    }

    async fn finish(mut self) -> Capture {
        let _ = tokio::time::timeout(DRAIN_GRACE, &mut self.task).await;
        self.task.abort();
        std::mem::take(&mut *self.buf.lock().expect("drain buffer"))
    }
}

/// A command that starts a login shell, as `bash -lc '...'`. GPT models wrap commands this way
/// despite the tool description. On macOS the profile runs `path_helper`, which moved Homebrew's
/// `python3` behind `/usr/bin/python3`. A plain `bash -c` wrapper is allowed: it keeps PATH.
fn login_shell(command: &str) -> bool {
    let mut words = command.split_whitespace();
    let shell = words
        .next()
        .and_then(|w| w.rsplit('/').next())
        .is_some_and(|w| matches!(w, "bash" | "sh" | "zsh"));
    shell
        && words
            .take_while(|w| w.starts_with('-'))
            .any(|w| w == "--login" || (!w.starts_with("--") && w.contains('l')))
}

/// The first meaningful line of stderr, kept short enough to sit on one terminal row.
fn first_line(stderr: &str) -> Option<String> {
    let line = stderr.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(crate::frontend::one_line(line, 72))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here runs under the kernel policy; `tools::only_fs_bounds_bash` covers the
    /// modes that do not.
    fn sandboxed() -> Bounds {
        Bounds::new(crate::config::Confine::Fs, std::env::current_dir().unwrap())
    }

    async fn run(command: &str) -> Outcome {
        run_for(command, 10_000).await
    }

    /// A path under `$HOME`: the one place outside the root that is neither the temp directory
    /// nor a build cache, and the place the policy exists to protect. `None` when `$HOME` is
    /// unset, which leaves the sandbox tests with nothing to aim at.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn outside_root(name: &str) -> Option<std::path::PathBuf> {
        let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
        home.is_dir()
            .then(|| home.join(format!(".minima-sandbox-{name}-{}", std::process::id())))
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn quoted(path: &std::path::Path) -> String {
        format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
    }

    async fn run_for(command: &str, timeout_ms: u64) -> Outcome {
        let args = Args {
            command: command.to_string(),
            timeout_ms: Some(timeout_ms),
        };
        call(args, &Cancel::new(), &sandboxed())
            .await
            .expect("bash tool")
    }

    #[cfg(unix)]
    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Orphans are reaped by init, not instantly, so poll briefly before declaring a leak.
    #[cfg(unix)]
    async fn gone(pid: i32) -> bool {
        for _ in 0..50 {
            if !alive(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        false
    }

    #[cfg(unix)]
    fn pid_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("minima-bash-{name}-{}", std::process::id()))
    }

    #[cfg(unix)]
    fn read_pid(path: &std::path::Path) -> i32 {
        let pid = std::fs::read_to_string(path).expect("pid file");
        let _ = std::fs::remove_file(path);
        pid.trim().parse().expect("a pid")
    }

    #[test]
    fn a_login_shell_wrapper_is_detected() {
        for command in [
            "bash -lc 'ls'",
            "  /bin/bash -l -c 'ls'",
            "sh -lc ls",
            "zsh --login -c ls",
            "bash -el -c ls",
        ] {
            assert!(login_shell(command), "{command}");
        }
        for command in [
            "bash -c 'ls -l'",
            "bash -ec ls -l",
            "ls -l",
            "bashful -lc ls",
            "bash --noprofile -c ls",
            "bash script.sh -l",
        ] {
            assert!(!login_shell(command), "{command}");
        }
    }

    #[tokio::test]
    async fn a_login_shell_wrapper_is_refused_without_running() {
        let marker = std::env::temp_dir().join(format!("minima-login-{}", std::process::id()));
        let args = Args {
            command: format!("bash -lc 'touch {}'", marker.display()),
            timeout_ms: None,
        };
        let err = call(args, &Cancel::new(), &sandboxed())
            .await
            .expect_err("refused");
        assert_eq!(err.to_string(), LOGIN_SHELL);
        assert!(!marker.exists());
    }

    #[tokio::test]
    async fn a_clean_exit_has_no_note() {
        let out = run("echo hi").await;
        assert_eq!(out.note, None);
        assert!(out.body.contains("hi"));
    }

    #[tokio::test]
    async fn a_failing_command_reports_its_code_and_reason() {
        let out = run("echo 'bad flag' >&2; exit 3").await;
        assert_eq!(out.note.as_deref(), Some("exit 3: bad flag"));
        // The model still sees the status in the transcript, not only the user.
        assert!(out.body.contains("exit 3"), "body was {:?}", out.body);
    }

    #[tokio::test]
    async fn a_failing_command_without_stderr_still_reports_its_code() {
        let out = run("exit 4").await;
        assert_eq!(out.note.as_deref(), Some("exit 4"));
    }

    #[tokio::test]
    async fn stdout_without_a_newline_does_not_run_into_stderr() {
        let out = run("printf out; echo err >&2").await;
        assert!(out.body.starts_with("out\nerr\n"), "{:?}", out.body);
    }

    /// The case that motivated the note: a pipeline whose last stage succeeds, hiding the
    /// failure of an earlier one behind exit 0.
    #[tokio::test]
    async fn stderr_is_reported_even_when_the_pipeline_exits_zero() {
        let out = run("echo 'unknown primary' >&2 | wc -c").await;
        assert_eq!(out.note.as_deref(), Some("stderr: unknown primary"));
    }

    /// Capture is bounded while the command runs, not after it returns: by then a command that
    /// writes without stopping has already had the memory.
    #[tokio::test]
    async fn output_past_the_cap_is_bounded_and_keeps_both_ends() {
        let out = run("echo START; yes 0123456789 | head -c 2000000; echo END").await;

        assert!(
            out.body.len() < CAPTURE_CAP + 128,
            "captured {} bytes",
            out.body.len()
        );
        assert!(out.body.starts_with("START\n"), "{:?}", &out.body[..32]);
        assert!(out.body.trim_end().ends_with("END"), "the tail was lost");
        assert!(
            out.body.contains("bytes elided"),
            "the gap was not reported"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_timeout_kills_what_bash_started_and_keeps_the_output() {
        let pids = pid_file("timeout");
        let out = run_for(
            &format!(
                "echo partial; sleep 30 & echo $! > {}; wait",
                pids.display()
            ),
            500,
        )
        .await;

        assert_eq!(out.note.as_deref(), Some("timed out after 500ms"));
        assert!(out.body.contains("partial"), "body was {:?}", out.body);
        assert!(gone(read_pid(&pids)).await, "the background sleep survived");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_cancel_kills_what_bash_started() {
        let pids = pid_file("cancel");
        let cancel = Cancel::new();
        let args = Args {
            command: format!("sleep 30 & echo $! > {}; wait", pids.display()),
            timeout_ms: Some(10_000),
        };
        let trigger = tokio::spawn({
            let cancel = cancel.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                cancel.cancel();
            }
        });

        let out = call(args, &cancel, &sandboxed()).await.expect("bash tool");
        trigger.await.expect("trigger");
        assert_eq!(out.body, crate::tools::CANCELLED);
        assert!(gone(read_pid(&pids)).await, "the background sleep survived");
    }

    /// A background job keeps the pipe open after bash exits. That must not hold the call until
    /// the timeout. The kill at exit is tested in `tests/headless.rs`: calling it here would kill
    /// the groups of tests running in parallel.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_background_job_is_left_running_reported_and_kept_for_exit() {
        let started = std::time::Instant::now();
        let out = run_for("sleep 30 & echo $!", 10_000).await;
        assert!(started.elapsed() < Duration::from_secs(2), "{out:?}");
        assert_eq!(out.note.as_deref(), Some(LEFT_RUNNING));
        assert!(out.body.contains(LEFT_RUNNING), "the model is told too");

        let pid: i32 = out.body.lines().next().unwrap().parse().expect("a pid");
        assert!(alive(pid));
        // SAFETY: getpgid takes no pointers.
        let group = unsafe { libc::getpgid(pid) } as u32;
        assert!(groups().contains(&group));

        assert!(signal_group(group, SIGKILL));
        forget(group);
        assert!(gone(pid).await);
    }

    #[tokio::test]
    async fn a_call_that_leaves_nothing_running_is_not_kept() {
        let out = run("echo $$").await;
        let group: u32 = out
            .body
            .trim()
            .parse()
            .expect("bash's pid, which is its group id");
        assert!(!groups().contains(&group));
    }

    /// The policy bounds writes, not reads. A toolchain, its headers and its dependency sources
    /// all sit outside the root.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn reads_outside_the_root_are_allowed() {
        let out = run("ls /usr >/dev/null && echo READABLE").await;
        assert!(out.body.contains("READABLE"), "{out:?}");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn the_temp_directory_and_dev_null_stay_writable() {
        let out = run(
            "f=$(mktemp) && echo x >\"$f\" && echo y >/dev/null && rm \"$f\" \
                       && echo WRITABLE",
        )
        .await;
        assert!(out.body.contains("WRITABLE"), "{out:?}");
    }

    /// Landlock denies every rename across directories below ABI 2, so `mv` is how a floor that
    /// slipped to V1 would show itself.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_rename_across_directories_is_allowed() {
        let out = run(
            "d=$(mktemp -d) && mkdir \"$d/a\" \"$d/b\" && touch \"$d/a/x\" \
                       && mv \"$d/a/x\" \"$d/b/x\" && rm -rf \"$d\" && echo RENAMED",
        )
        .await;
        assert!(out.body.contains("RENAMED"), "{out:?}");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn writes_outside_the_root_are_denied() {
        let Some(path) = outside_root("write") else {
            return;
        };
        let out = run(&format!("touch {} && echo WROTE", quoted(&path))).await;
        let created = path.exists();
        let _ = std::fs::remove_file(&path);
        assert!(!created, "a write reached {}", path.display());
        assert!(!out.body.contains("WROTE"), "{out:?}");
    }

    /// A denied write reaches the model as whatever the command printed, which never names the
    /// policy. The note is what tells it the difference between a broken machine and a bound one.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_denied_write_says_which_policy_denied_it() {
        let Some(path) = outside_root("denied-note") else {
            return;
        };
        let out = run(&format!("touch {}", quoted(&path))).await;
        let _ = std::fs::remove_file(&path);
        let note = out.note.unwrap_or_default();
        assert!(note.contains("--writable"), "{note}");
        assert!(out.body.contains("--confine fs"), "{:?}", out.body);
    }

    /// The note is for the mode that can deny. Under `paths` nothing bounds `bash`, so a
    /// permission error is the filesystem's own and the policy has nothing to say about it.
    #[tokio::test]
    async fn an_unsandboxed_permission_error_gets_no_note() {
        let args = Args {
            command: "touch /minima-not-writable 2>&1".into(),
            timeout_ms: Some(10_000),
        };
        let bounds = Bounds::new(
            crate::config::Confine::Paths,
            std::env::current_dir().unwrap(),
        );
        let out = call(args, &Cancel::new(), &bounds)
            .await
            .expect("bash tool");
        assert!(!out.body.contains("--writable"), "{:?}", out.body);
    }

    /// `--writable` widens the kernel policy and nothing else: the named directory takes a
    /// write, the sibling beside it does not.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn only_the_named_directory_is_added() {
        let Some(base) = outside_root("writable") else {
            return;
        };
        let (granted, denied) = (base.join("granted"), base.join("denied"));
        std::fs::create_dir_all(&granted).unwrap();
        std::fs::create_dir_all(&denied).unwrap();

        let mut bounds = sandboxed();
        bounds
            .writable
            .push(std::fs::canonicalize(&granted).unwrap());
        let args = Args {
            command: format!("touch {}/a; touch {}/b", quoted(&granted), quoted(&denied)),
            timeout_ms: Some(10_000),
        };
        let out = call(args, &Cancel::new(), &bounds)
            .await
            .expect("bash tool");

        let (added, sibling) = (granted.join("a").exists(), denied.join("b").exists());
        let _ = std::fs::remove_dir_all(&base);
        assert!(added, "the --writable directory was denied: {out:?}");
        assert!(
            !sibling,
            "a write reached a sibling of the --writable directory"
        );
    }

    /// Landlock handles `Truncate` only from ABI 3. Below it the read grant on `/` still permits
    /// `: > file` anywhere, which destroys the file without ever writing to it.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn truncating_a_file_outside_the_root_is_denied() {
        let Some(path) = outside_root("truncate") else {
            return;
        };
        std::fs::write(&path, "kept").unwrap();
        let out = run(&format!(": > {}", quoted(&path))).await;
        let after = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        assert_eq!(after, "kept", "a truncate reached the file: {out:?}");
    }

    /// Resolution through `PATH` would let a shim named `sandbox-exec` run the command
    /// unconfined, with `preflight` reading its exit 0 as a working sandbox.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_sandbox_is_spawned_by_absolute_path() {
        let command = sandbox_command(
            &Bounds::new(crate::config::Confine::Fs, std::path::PathBuf::from("/")),
            "exit 0",
        )
        .unwrap();
        let program = command.as_std().get_program();
        assert_eq!(program, SANDBOX_EXEC);
        assert!(
            std::path::Path::new(SANDBOX_EXEC).is_file(),
            "{SANDBOX_EXEC} is missing"
        );
    }

    #[tokio::test]
    async fn preflight_installs_the_sandbox() {
        preflight(&sandboxed())
            .await
            .expect("the sandbox should install on a supported platform");
    }
}
