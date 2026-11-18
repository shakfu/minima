//! `-r`: a family of agents through the built binary, since spawn, mail and shutdown cross
//! processes.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const EXE: &str = env!("CARGO_BIN_EXE_minima");

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

/// A child's final answer reaches its parent's mailbox.
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
                &format!("'{EXE}' -r spawn --name kid --mock child.json -- 'say hi'"),
                10,
            ),
            bash_turn(1, &format!("'{EXE}' -r recv --wait 20 > got.txt"), 10),
            serde_json::json!([{"text": "done"}]),
        ],
    );
    let out = minima(&dir, &["-r", "--mock", "root.json", "-p", "go"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = std::fs::read_to_string(dir.join("got.txt")).expect("recv output");
    assert!(
        got.contains("from kid") && got.contains("hello from kid"),
        "{got}"
    );
}

/// The root's precheck crosses the budget: the child, mid-command, is stopped with its command,
/// and the root exits with the reason.
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
    let registry = std::fs::read_dir(dir.join("state/minima/family"))
        .unwrap()
        .flatten()
        .map(|e| std::fs::read_to_string(e.path().join("registry.json")).unwrap())
        .next()
        .expect("the root's family");
    assert!(
        !registry.contains(kid.trim()),
        "the child was not pruned: {registry}"
    );
}

/// `agents` outside any agent finds the family by directory, and says when there is none.
#[test]
fn agents_outside_a_family_says_so() {
    let dir = scratch("none");
    let out = minima(&dir, &["-r", "agents"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no family rooted at"));
}
