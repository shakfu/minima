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

use super::Outcome;
use crate::cancel::Cancel;
use crate::config::TOOL_OUTPUT_CAP;

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

pub async fn call(args: Args, cancel: &Cancel) -> Result<Outcome> {
    if login_shell(&args.command) {
        bail!(LOGIN_SHELL);
    }
    let limit = args
        .timeout_ms
        .map_or(DEFAULT_TIMEOUT, Duration::from_millis)
        .min(MAX_TIMEOUT);

    let mut command = Command::new("bash");
    command
        .arg("-c")
        .arg(&args.command)
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

    async fn run(command: &str) -> Outcome {
        run_for(command, 10_000).await
    }

    async fn run_for(command: &str, timeout_ms: u64) -> Outcome {
        let args = Args {
            command: command.to_string(),
            timeout_ms: Some(timeout_ms),
        };
        call(args, &Cancel::new()).await.expect("bash tool")
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
        let err = call(args, &Cancel::new()).await.expect_err("refused");
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

        let out = call(args, &cancel).await.expect("bash tool");
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
}
