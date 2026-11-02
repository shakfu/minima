//! The REPL's start, driven on a pseudo-terminal by `tests/fixtures/pty_driver.py`, because the
//! REPL refuses to run without one.

#![cfg(unix)]

use std::process::Command;

/// What the REPL drew, without escapes or whitespace, once `until` appeared and `settle` seconds
/// passed, and Ctrl-D ended it.
fn repl(name: &str, until: &str, settle: &str, words: &[&str]) -> String {
    let dir = std::env::temp_dir().join(format!("minima-repl-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    let output = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/pty_driver.py"
        ))
        .args(["--until", until, "--settle", settle, "--"])
        .arg(env!("CARGO_BIN_EXE_minima"))
        .arg("--mock")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/mock/say-hi.json"))
        .args(["--no-color", "--root"])
        .arg(&dir)
        .args(words)
        .env("XDG_CONFIG_HOME", &dir)
        .output()
        .expect("python3 is required for the REPL tests");
    let _ = std::fs::remove_dir_all(&dir);
    let drawn = String::from_utf8_lossy(&output.stdout).replace(char::is_whitespace, "");
    assert!(output.status.success(), "REPL run failed: {drawn}");
    drawn
}

/// The words are one prompt, echoed and answered before any key is pressed, and only once.
#[test]
fn a_positional_prompt_is_the_first_turn() {
    let drawn = repl("first", "hi from the mock", "0.5", &["hello", "there"]);
    assert!(drawn.contains(">hellothere"), "{drawn}");
    assert!(drawn.contains("hifromthemock"), "{drawn}");
    assert!(!drawn.contains("secondanswer"), "{drawn}");
}

#[test]
fn without_one_the_repl_waits_for_input() {
    let drawn = repl("none", "minima", "1", &[]);
    assert!(!drawn.contains("hifromthemock"), "{drawn}");
}
