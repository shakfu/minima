//! `-r`: a family of agents through the built binary, since spawn, mail and shutdown cross
//! processes.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const EXE: &str = env!("CARGO_BIN_EXE_minima");

/// Waits up to 10 s for the child's `bash` to record its pid. Bounded, so a child whose `bash`
/// never runs fails the test at once instead of at the root's 120 s tool timeout.
const AWAIT_PID: &str = "for _ in $(seq 100); do [ -s child-bash.pid ] && break; sleep 0.1; done";

fn bash_turn(i: usize, command: &str, total_tokens: u32) -> serde_json::Value {
    let arguments = serde_json::json!({ "command": command }).to_string();
    serde_json::json!([
        {"tool_call": {"index": 0, "id": format!("c{i}"), "name": "bash", "arguments": arguments}},
        {"usage": {"prompt_tokens": total_tokens, "completion_tokens": 0, "total_tokens": total_tokens}},
    ])
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("minima-recurse-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn write(dir: &Path, name: &str, turns: &[serde_json::Value]) {
    std::fs::write(
        dir.join(name),
        serde_json::Value::from(turns.to_vec()).to_string(),
    )
    .expect("writing a mock script");
}

fn minima(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(EXE)
        .current_dir(dir)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .args(args)
        .output()
        .expect("running minima")
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .status()
        .is_ok_and(|s| s.success())
}

/// Polls for up to five seconds: a child notices its parent's exit within one.
fn dies(pid: &str) -> bool {
    (0..50).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        !alive(pid)
    })
}

/// A child's final answer reaches its parent through `wait`. The task starts with `-`, which a
/// flag parser would take for an option.
#[test]
fn a_spawned_child_answers_its_parent() {
    let dir = scratch("answer");
    write(
        &dir,
        "child.json",
        &[serde_json::json!([{"text": "hello from kid"}])],
    );
    write(
        &dir,
        "root.json",
        &[
            bash_turn(
                0,
                &format!("'{EXE}' -r spawn --name kid --mock child.json -- '-say hi' > kid.id"),
                10,
            ),
            bash_turn(
                1,
                &format!("'{EXE}' -r wait kid --timeout 20 > got.txt"),
                10,
            ),
            serde_json::json!([{"text": "done"}]),
        ],
    );
    let out = minima(
        &dir,
        &[
            "-r",
            "--budget-tokens",
            "100000",
            "--mock",
            "root.json",
            "-p",
            "go",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = std::fs::read_to_string(dir.join("got.txt")).expect("wait output");
    assert_eq!(got.trim(), "hello from kid");
    // Saved under its member id, so `--resume ID` opens it.
    let kid = std::fs::read_to_string(dir.join("kid.id")).expect("child id");
    let sessions = dir.join("config/minima/sessions");
    let saved = std::fs::read_dir(&sessions)
        .expect("sessions directory")
        .flatten()
        .any(|d| d.path().join(format!("{}.json", kid.trim())).exists());
    assert!(
        saved,
        "no session named {} under {}",
        kid.trim(),
        sessions.display()
    );
}

/// A family needs a budget: nothing else bounds what its members spend.
#[test]
fn a_family_without_a_budget_is_refused() {
    let dir = scratch("no-budget");
    write(&dir, "root.json", &[serde_json::json!([{"text": "done"}])]);
    let out = minima(&dir, &["-r", "--mock", "root.json", "-p", "go"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--budget-tokens"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A child stops when its parent exits, with the command it was running: no orphans.
#[test]
fn a_child_stops_when_its_parent_exits() {
    let dir = scratch("orphan");
    write(
        &dir,
        "child.json",
        &[
            bash_turn(0, "echo $$ > child-bash.pid; sleep 30", 10),
            serde_json::json!([{"text": "late"}]),
        ],
    );
    write(
        &dir,
        "root.json",
        &[
            bash_turn(
                0,
                &format!(
                    "'{EXE}' -r spawn --name kid --mock child.json -- work; \
                     {AWAIT_PID}"
                ),
                10,
            ),
            serde_json::json!([{"text": "leaving"}]),
        ],
    );
    let out = minima(
        &dir,
        &[
            "-r",
            "--budget-tokens",
            "100000",
            "--mock",
            "root.json",
            "-p",
            "go",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bash = std::fs::read_to_string(dir.join("child-bash.pid")).expect("child's bash pid");
    assert!(dies(bash.trim()), "the child's command outlived its parent");
}

/// `shutdown` run by an agent stops its children and leaves the agent running.
#[test]
fn an_agents_shutdown_spares_the_agent() {
    let dir = scratch("shutdown");
    write(
        &dir,
        "child.json",
        &[
            bash_turn(0, "echo $$ > child-bash.pid; sleep 30", 10),
            serde_json::json!([{"text": "late"}]),
        ],
    );
    write(
        &dir,
        "root.json",
        &[
            bash_turn(
                0,
                &format!(
                    "'{EXE}' -r spawn --name kid --mock child.json -- work; \
                     {AWAIT_PID}; '{EXE}' -r shutdown"
                ),
                10,
            ),
            serde_json::json!([{"text": "still here"}]),
        ],
    );
    let out = minima(
        &dir,
        &[
            "-r",
            "--budget-tokens",
            "100000",
            "--mock",
            "root.json",
            "-p",
            "go",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("still here"));
    let bash = std::fs::read_to_string(dir.join("child-bash.pid")).expect("child's bash pid");
    assert!(
        dies(bash.trim()),
        "the child's command outlived the shutdown"
    );
}

/// The root's precheck crosses the budget: the child, mid-command, is stopped with its command
/// and marked ended, and the root exits with the reason.
#[test]
fn crossing_the_budget_shuts_the_family_down() {
    let dir = scratch("budget");
    write(
        &dir,
        "child.json",
        &[
            bash_turn(0, "echo $$ > child-bash.pid; sleep 30", 10),
            serde_json::json!([{"text": "late"}]),
        ],
    );
    write(
        &dir,
        "root.json",
        &[
            bash_turn(
                0,
                &format!("'{EXE}' -r spawn --name kid --mock child.json -- work > kid.id; sleep 1"),
                10,
            ),
            bash_turn(1, "true", 35_000),
            serde_json::json!([{"text": "not reached"}]),
        ],
    );
    let out = minima(
        &dir,
        &[
            "-r",
            "--budget-tokens",
            "40000",
            "--context",
            "60000",
            "--mock",
            "root.json",
            "-p",
            "go",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("family token budget"), "{stderr}");

    let kid = std::fs::read_to_string(dir.join("kid.id")).expect("child id");
    let bash = std::fs::read_to_string(dir.join("child-bash.pid")).expect("child's bash pid");
    assert!(
        !alive(bash.trim()),
        "the child's command outlived the shutdown"
    );
    let registry: serde_json::Value = std::fs::read_dir(dir.join("state/minima/family"))
        .unwrap()
        .flatten()
        .map(|e| {
            serde_json::from_str(&std::fs::read_to_string(e.path().join("registry.json")).unwrap())
                .unwrap()
        })
        .next()
        .expect("the root's family");
    // Kept, not pruned, while the root ran: a parent may still read a child's answer or log.
    let child = registry["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == kid.trim())
        .expect("the child in the registry");
    assert!(
        child["ended_at"].is_u64(),
        "the child was not marked ended: {child}"
    );
}

/// A child crossing the budget stops its own subtree, not its parent: the root is not signalled,
/// reads the child's failure through `wait`, and is then refused by its own check.
/// 93,000 of the 95,000 line leaves less than any request's input plus the 4,096-token floor.
#[test]
fn a_child_crossing_the_budget_spares_its_parent() {
    let dir = scratch("child-budget");
    write(
        &dir,
        "child.json",
        &[
            bash_turn(0, "true", 93_000),
            serde_json::json!([{"text": "not reached"}]),
        ],
    );
    write(
        &dir,
        "root.json",
        &[
            // One command, so the root sends no request between the spawn and the wait.
            bash_turn(
                0,
                &format!(
                    "'{EXE}' -r spawn --name kid --mock child.json -- work; \
                     '{EXE}' -r wait kid --timeout 20 > got.txt"
                ),
                10,
            ),
            serde_json::json!([{"text": "not reached"}]),
        ],
    );
    let out = minima(
        &dir,
        &[
            "-r",
            "--budget-tokens",
            "100000",
            "--context",
            "60000",
            "--mock",
            "root.json",
            "-p",
            "go",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 143 would be the child's SIGTERM.
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("family token budget"), "{stderr}");
    let got = std::fs::read_to_string(dir.join("got.txt")).expect("wait output");
    assert!(got.contains("failed: family token budget"), "{got}");
}

/// `agents` outside any agent finds the family by directory, and says when there is none.
#[test]
fn agents_outside_a_family_says_so() {
    let dir = scratch("none");
    let out = minima(&dir, &["-r", "agents"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no family rooted at"));
}

/// Under `--sandbox`, a child's write outside the root is refused: the kernel policy on the
/// parent's `bash` confines every process started from it, the child included. The family
/// directory is outside both the root and `$TMPDIR`, so `spawn` and `wait` work only through the
/// grant that adds it to the writable set.
#[test]
fn a_sandboxed_family_writes_its_own_files_and_nothing_else() {
    let dir = scratch("sandbox");
    let root = dir.join("root");
    let tmp = dir.join("tmp");
    let outside = dir.join("outside");
    for d in [&root, &tmp, &outside] {
        std::fs::create_dir_all(d).unwrap();
    }
    let target = outside.join("child-wrote");
    write(
        &root,
        "child.json",
        &[
            bash_turn(
                0,
                &format!(
                    "if touch '{}' 2>/dev/null; then echo wrote; else echo denied; fi > child-result.txt",
                    target.display()
                ),
                10,
            ),
            serde_json::json!([{"text": "done"}]),
        ],
    );
    write(
        &root,
        "root.json",
        &[
            bash_turn(
                0,
                &format!(
                    "'{EXE}' -r spawn --name kid --mock child.json -- work && \\
                     '{EXE}' -r wait kid --timeout 20 > got.txt 2>&1"
                ),
                10,
            ),
            serde_json::json!([{"text": "finished"}]),
        ],
    );
    let out = Command::new(EXE)
        .current_dir(&root)
        .env("TMPDIR", &tmp)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .args([
            "-r",
            "--budget-tokens",
            "100000",
            "--sandbox",
            "--mock",
            "root.json",
            "-p",
            "go",
        ])
        .output()
        .expect("running minima");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = std::fs::read_to_string(root.join("got.txt")).expect("wait output");
    assert_eq!(got.trim(), "done", "{got}");
    let result = std::fs::read_to_string(root.join("child-result.txt")).expect("child's result");
    assert_eq!(result.trim(), "denied");
    assert!(!target.exists());
}

/// A sandboxed run of `root.json` in `root`, with `$TMPDIR` and the state directory outside it.
fn sandboxed(dir: &Path) -> std::process::Output {
    let root = dir.join("root");
    let tmp = dir.join("tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    Command::new(EXE)
        .current_dir(&root)
        .env("TMPDIR", &tmp)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .args([
            "-r",
            "--budget-tokens",
            "100000",
            "--sandbox",
            "--mock",
            "root.json",
            "-p",
            "go",
        ])
        .output()
        .expect("running minima")
}

/// The parent watch and an agent's `shutdown` both signal across sandboxes: a sandboxed child
/// probes its parent, and a sandboxed `bash` stops a child another `bash` started.
#[test]
fn a_sandboxed_child_stops_with_its_parent_and_on_shutdown() {
    // `ps` cannot run in the macOS sandbox, and `kill -0` there refuses a live process outside
    // it, so only "No such process" means gone.
    let check = "; sleep 0.5; if kill -0 \"$(cat child-bash.pid)\" 2>&1 | grep -q 'No such process'; \
                 then echo dead; else echo alive; fi > after.txt";
    for (case, last) in [
        ("orphan", String::new()),
        ("shutdown", format!("; '{{EXE}}' -r shutdown{check}")),
    ] {
        let dir = scratch(&format!("sandbox-{case}"));
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        write(
            &root,
            "child.json",
            &[
                bash_turn(0, "echo $$ > child-bash.pid; sleep 30", 10),
                serde_json::json!([{"text": "late"}]),
            ],
        );
        let last = last.replace("{EXE}", EXE);
        write(
            &root,
            "root.json",
            &[
                bash_turn(
                    0,
                    &format!(
                        "'{EXE}' -r spawn --name kid --mock child.json -- work; \\
                         {AWAIT_PID}"
                    ),
                    10,
                ),
                bash_turn(1, &format!("true{last}"), 10),
                serde_json::json!([{"text": "finished"}]),
            ],
        );
        let out = sandboxed(&dir);
        assert!(
            out.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let bash = std::fs::read_to_string(root.join("child-bash.pid")).expect("child's bash pid");
        assert!(dies(bash.trim()), "{case}: the child's command outlived it");
        if case == "shutdown" {
            // Stopped by `shutdown` while the root ran, not by the root's exit.
            let after =
                std::fs::read_to_string(root.join("after.txt")).expect("check after shutdown");
            assert_eq!(after.trim(), "dead");
        }
    }
}
