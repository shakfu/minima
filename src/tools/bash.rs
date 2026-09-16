//! Cancellation and the timeout both work by dropping the wait future. `kill_on_drop` then
//! reaps the child, so neither path can leave a process behind.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::process::Command;

use super::Outcome;
use crate::cancel::Cancel;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Shell command to run.
    pub command: String,
    /// Timeout in milliseconds. Defaults to 120000, capped at 600000.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

pub async fn call(args: Args, cancel: &Cancel) -> Result<Outcome> {
    let limit = args
        .timeout_ms
        .map_or(DEFAULT_TIMEOUT, Duration::from_millis)
        .min(MAX_TIMEOUT);

    let child = Command::new("bash")
        .arg("-lc")
        .arg(&args.command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("spawning: {}", args.command))?;

    let finished = tokio::select! {
        result = tokio::time::timeout(limit, child.wait_with_output()) => result,
        () = cancel.cancelled() => return Ok("cancelled by the user".to_string().into()),
    };

    let Ok(output) = finished else {
        let note = format!("timed out after {}ms", limit.as_millis());
        return Ok(Outcome {
            body: note.clone(),
            note: Some(note),
        });
    };
    let output = output.context("waiting for the command")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut body = String::from_utf8_lossy(&output.stdout).into_owned();
    body.push_str(&stderr);

    // A non-zero exit is information for the model, not a tool failure. The note is what the user
    // sees, so it leads with the reason rather than the whole output.
    //
    // Exit status alone is not enough. In a pipeline the status belongs to the last stage, so
    // `find -printf ... | wc -c` exits 0 while find's error goes to stderr unseen. Anything on
    // stderr is therefore worth a line, whatever the exit code says.
    let reason = first_line(&stderr);
    let note = match (output.status.code(), reason) {
        (Some(0), None) => None,
        (Some(0), Some(line)) => Some(format!("stderr: {line}")),
        (Some(code), Some(line)) => Some(format!("exit {code}: {line}")),
        (Some(code), None) => Some(format!("exit {code}")),
        (None, _) => Some("killed by a signal".to_string()),
    };

    if body.is_empty() {
        body.push_str("(no output)");
    }
    if let Some(note) = &note {
        body.push_str(&format!("\n({note})"));
    }
    Ok(Outcome { body, note })
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
        let args = Args {
            command: command.to_string(),
            timeout_ms: Some(10_000),
        };
        call(args, &Cancel::new()).await.expect("bash tool")
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

    /// The case that motivated the note: a pipeline whose last stage succeeds, hiding the
    /// failure of an earlier one behind exit 0.
    #[tokio::test]
    async fn stderr_is_reported_even_when_the_pipeline_exits_zero() {
        let out = run("echo 'unknown primary' >&2 | wc -c").await;
        assert_eq!(out.note.as_deref(), Some("stderr: unknown primary"));
    }
}
