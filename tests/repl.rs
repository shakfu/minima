//! The REPL's start, driven on a pseudo-terminal by `tests/fixtures/pty_driver.py`, because the
//! REPL refuses to run without one.

#![cfg(unix)]

use std::process::Command;

/// What the REPL drew, without escapes or whitespace, once `until` appeared and `settle` seconds
/// passed, and Ctrl-D ended it.
fn repl(name: &str, until: &str, settle: &str, words: &[&str]) -> String {
    let mock = concat!(env!("CARGO_MANIFEST_DIR"), "/mock/say-hi.json");
    repl_with(name, mock.as_ref(), until, settle, words, &[])
}

/// `repl`, replaying `mock` with `env` set. Herdr's variables are cleared first, so a test run inside a Herdr
/// pane does not report to it.
fn repl_with(
    name: &str,
    mock: &std::path::Path,
    until: &str,
    settle: &str,
    words: &[&str],
    env: &[(&str, &std::ffi::OsStr)],
) -> String {
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
        .arg(mock)
        .args(["--no-color", "--root"])
        .arg(&dir)
        .args(words)
        .env("XDG_CONFIG_HOME", &dir)
        .env_remove("HERDR_ENV")
        .env_remove("HERDR_PANE_ID")
        .env_remove("HERDR_BIN_PATH")
        .env_remove("HERDR_SOCKET_PATH")
        .envs(env.iter().copied())
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

/// A fake `herdr` logs each call. A turn reports working then idle, with the command that
/// resumes the saved session, and the quit releases. The
/// turn sleeps 1 s in `bash`, so `working` is not coalesced into the `idle` after it.
#[test]
fn reports_state_to_herdr_and_releases_on_quit() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("minima-herdr-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("log");
    let bin = dir.join("herdr");
    std::fs::write(
        &bin,
        format!("#!/bin/sh\necho \"$*\" >> '{}'\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mock = dir.join("mock.json");
    std::fs::write(
        &mock,
        r#"[[{"tool_call": {"index": 0, "id": "c1", "name": "bash",
                            "arguments": "{\"command\":\"sleep 1\"}"}}],
            [{"text": "slept\n"}]]"#,
    )
    .unwrap();

    repl_with(
        "herdr",
        &mock,
        "slept",
        "0.5",
        &["hello"],
        &[
            ("HERDR_ENV", "1".as_ref()),
            ("HERDR_PANE_ID", "p7".as_ref()),
            ("HERDR_BIN_PATH", bin.as_os_str()),
            ("HERDR_SOCKET_PATH", dir.join("sock").as_os_str()),
        ],
    );
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    // Each line: pane <verb> [--state S] --seq N --source minima --agent minima
    // [--agent-session-id ID --agent-session-path PATH] p7 [-- RESUME...].
    // `repl_with`'s root, resolved as minima resolves it.
    let root = std::fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("minima-repl-herdr-{}", std::process::id()));
    let mut seqs = Vec::new();
    let mut shape = Vec::new();
    for line in calls.lines() {
        let (report, resume) = line.split_once(" -- ").unwrap_or((line, ""));
        let words: Vec<&str> = report.split(' ').collect();
        let at = words.iter().position(|w| *w == "--seq").expect(line);
        seqs.push(words[at + 1].parse::<u64>().expect(line));
        assert_eq!(
            &words[at + 2..at + 6],
            ["--source", "minima", "--agent", "minima"]
        );
        assert_eq!(words.last(), Some(&"p7"), "{line}");
        let mut entry = words[..at].join(" ");
        if !resume.is_empty() {
            let id = &words[at + 7];
            assert_eq!(words[at + 6], "--agent-session-id", "{line}");
            let expected = format!("minima --root {} --resume {id}", root.display());
            assert_eq!(resume, expected, "{line}");
            entry.push_str(" +resume");
        }
        shape.push(entry);
    }
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "{calls}");
    // The idle before the first prompt may be coalesced away by the working that follows it.
    // Nothing is saved before the turn ends, so only the idle after it can be resumed.
    let shape: Vec<&str> = shape.iter().map(String::as_str).collect();
    let tail = [
        "pane report-agent --state working",
        "pane report-agent --state idle +resume",
        "pane release-agent",
    ];
    assert!(shape.ends_with(&tail), "{calls}");
}

/// `/model` switches in session, then the status bar and the confirmation name the new model.
#[test]
fn model_switches_in_session() {
    let drawn = repl("model", "context", "0.5", &["/model", "other-model"]);
    assert!(drawn.contains("model:other-model,context128k"), "{drawn}");
    assert!(!drawn.contains("hifromthemock"), "no turn ran: {drawn}");
}
