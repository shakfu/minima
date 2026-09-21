//! What a script sees from `-p`, and what the process leaves behind: only the built binary can
//! show either.

#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::time::Duration;

/// A mock script whose turns each make one `bash` call, then a final answer.
fn bash_turns(commands: &[&str]) -> String {
    let mut turns: Vec<_> = commands
        .iter()
        .enumerate()
        .map(|(i, command)| {
            let arguments = serde_json::json!({ "command": command }).to_string();
            serde_json::json!([{"tool_call": {
                "index": 0, "id": format!("c{i}"), "name": "bash", "arguments": arguments,
            }}])
        })
        .collect();
    turns.push(serde_json::json!([{"text": "done"}]));
    serde_json::Value::Array(turns).to_string()
}

struct Run {
    dir: PathBuf,
    child: Child,
    stderr: BufReader<ChildStderr>,
}

impl Run {
    /// Tools run in a fresh directory, so relative pid files land there.
    fn start(name: &str, script: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("minima-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let path = dir.join("script.json");
        std::fs::write(&path, script).expect("writing the mock script");

        let mut child = Command::new(env!("CARGO_BIN_EXE_minima"))
            .env("XDG_CONFIG_HOME", &dir)
            .current_dir(&dir)
            .arg("--mock")
            .arg(&path)
            .args(["-p", "go"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("running minima");
        let stderr = BufReader::new(child.stderr.take().expect("stderr"));
        Self { dir, child, stderr }
    }

    /// Blocks until the tool line for the `n`th bash call is printed, just before it runs.
    fn wait_for_bash_call(&mut self, n: usize) {
        let mut seen = 0;
        let mut line = String::new();
        while seen < n {
            line.clear();
            let read = self.stderr.read_line(&mut line).expect("reading stderr");
            assert!(read > 0, "minima exited before bash call {n}");
            seen += usize::from(line.starts_with("$ "));
        }
    }

    fn signal(&self, name: &str) {
        let sent = Command::new("kill")
            .args([&format!("-{name}"), &self.child.id().to_string()])
            .status()
            .expect("running kill");
        assert!(sent.success());
    }

    fn wait(&mut self) -> Option<i32> {
        self.child.wait().expect("waiting for minima").code()
    }

    fn pid(&self, file: &str) -> i32 {
        let path = self.dir.join(file);
        wait_until(|| std::fs::read_to_string(&path).is_ok_and(|s| s.ends_with('\n')));
        let text = std::fs::read_to_string(&path).expect("pid file");
        text.trim().parse().expect("a pid")
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn wait_until(mut done: impl FnMut() -> bool) -> bool {
    for _ in 0..100 {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Orphans are reaped by init, not instantly, so this polls.
fn gone(pid: i32) -> bool {
    // SAFETY: signal 0 only checks that the process exists.
    wait_until(|| unsafe { libc::kill(pid, 0) } != 0)
}

fn group_gone(pgid: i32) -> bool {
    // SAFETY: signal 0 only checks that the group has a member.
    wait_until(|| unsafe { libc::killpg(pgid, 0) } != 0)
}

#[test]
fn ctrl_c_during_a_tool_exits_130() {
    let mut run = Run::start("ctrl-c", &bash_turns(&["sleep 20"]));
    run.wait_for_bash_call(1);
    run.signal("INT");
    assert_eq!(run.wait(), Some(130));
}

/// Left running by the first call, used by the second, killed when the run ends.
#[test]
fn a_background_job_outlives_its_call_but_not_the_run() {
    let mut run = Run::start(
        "background",
        &bash_turns(&[
            "sleep 30 & echo $! > job.pid",
            "kill -0 $(cat job.pid) && echo alive > seen.txt",
        ]),
    );
    assert_eq!(run.wait(), Some(0));
    assert!(
        run.dir.join("seen.txt").exists(),
        "the job died with its call"
    );
    assert!(
        gone(run.pid("job.pid")),
        "the background job outlived minima"
    );
}

/// SIGTERM and SIGHUP exit through `process::exit`, which skips the guard that kills jobs on a
/// normal return. The call in flight when the signal lands is killed too.
#[test]
fn sigterm_kills_background_jobs_and_the_running_call() {
    let mut run = Run::start(
        "sigterm",
        &bash_turns(&[
            "sleep 30 & echo $! > job.pid",
            "echo $$ > call.pid; sleep 20",
        ]),
    );
    run.wait_for_bash_call(2);
    let job = run.pid("job.pid");
    let call = run.pid("call.pid");

    run.signal("TERM");
    assert_eq!(run.wait(), Some(143));
    assert!(gone(job), "the background job outlived minima");
    assert!(group_gone(call), "the running call outlived minima");
}

/// `--json` output as a caller reads it: the last stdout line is the result, the exit code agrees
/// with it, and an error is not repeated on stderr.
#[test]
fn json_ends_in_a_result_matching_the_exit_code() {
    for (name, script, code, outcome) in [
        ("json-ok", bash_turns(&["true"]), 0, "complete"),
        ("json-error", "[]".to_string(), 1, "error"),
    ] {
        let dir = std::env::temp_dir().join(format!("minima-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let path = dir.join("script.json");
        std::fs::write(&path, script).expect("writing the mock script");

        let out = Command::new(env!("CARGO_BIN_EXE_minima"))
            .env("XDG_CONFIG_HOME", &dir)
            .current_dir(&dir)
            .arg("--mock")
            .arg(&path)
            .args(["--json", "-p", "go"])
            .output()
            .expect("running minima");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(out.status.code(), Some(code), "{name}");
        assert!(out.stderr.is_empty(), "{name}: {:?}", out.stderr);
        let stdout = String::from_utf8(out.stdout).expect("utf-8");
        let last: serde_json::Value =
            serde_json::from_str(stdout.lines().last().expect("a line")).expect("JSON");
        assert_eq!(last["type"], "result", "{name}");
        assert_eq!(last["outcome"], outcome, "{name}");
    }
}
