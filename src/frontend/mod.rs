//! Presentation, kept out of the agent loop.
//!
//! Sync on purpose: every method is a write to an already-open stream, so making the trait async
//! would buy nothing and cost object safety.

pub mod headless;
mod history;
pub mod json;
mod markdown;
pub mod repl;
mod screen;

use std::time::Duration;

use crate::provider::Usage;

pub trait Frontend {
    /// A fragment of assistant text, as it streams.
    fn text(&mut self, delta: &str);

    fn tool_start(&mut self, name: &str, arguments: &str);

    /// `note` is set when the tool ran but reported a problem, such as a non-zero exit. `ok` is
    /// false only when the tool itself failed, and `note` is then the error's root cause.
    fn tool_end(&mut self, body: &str, note: Option<&str>, ok: bool);

    fn retry(&mut self, attempt: u32, delay: Duration);

    fn turn_end(&mut self, usage: Usage);

    /// Older messages were replaced by a summary. Both counts are estimated tokens.
    fn compacted(&mut self, before: u32, after: u32);

    fn cancelled(&mut self);
}

/// `1.2k`, `3.45M`: short enough for the status bar.
pub fn count(n: u64) -> String {
    let (value, unit, places) = match n {
        0..1_000 => return n.to_string(),
        1_000..1_000_000 => (n as f64 / 1e3, "k", 1),
        _ => (n as f64 / 1e6, "M", 2),
    };
    let text = format!("{value:.places$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}{unit}")
}

/// The line both text frontends print after a compaction.
pub fn compacted(before: u32, after: u32) -> String {
    format!(
        "compacted: ~{} -> ~{} tokens",
        count(before.into()),
        count(after.into())
    )
}

/// Drops control characters except newline and tab. Text a model echoes from a file it read can
/// carry escape sequences that retitle the terminal or write to the clipboard.
pub fn printable(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

/// Tool arguments and results are echoed as one line, with control characters flattened. The full
/// text is in the transcript the model sees; the frontend only has to show that something ran.
pub fn one_line(text: &str, width: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = flat.trim();
    if trimmed.chars().count() <= width {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(width.saturating_sub(3)).collect();
    format!("{head}...")
}

/// A tool call as `[tool] read src/lib.rs:495-994` rather than its JSON. Unknown tools and
/// arguments that do not parse fall back to the raw form.
pub fn describe(name: &str, arguments: &str, width: usize) -> String {
    let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let path = args["path"].as_str();
    let root = std::env::current_dir().unwrap_or_default();
    let root = root.to_string_lossy();
    let path = path.map(|p| relative(p, &root));
    let detail = match (name, path, args["command"].as_str()) {
        ("bash", _, Some(command)) => without_cd(command, &root).to_string(),
        ("read", Some(path), _) => {
            let start = args["offset"].as_u64().unwrap_or(1).max(1);
            match args["limit"].as_u64() {
                Some(limit) => format!("{path}:{start}-{}", start + limit.max(1) - 1),
                None if start > 1 => format!("{path}:{start}-"),
                None => path.to_string(),
            }
        }
        ("write" | "edit", Some(path), _) => path.to_string(),
        _ => arguments.to_string(),
    };
    one_line(&format!("[tool] {name} {detail}"), width)
}

/// A path under the working directory, relative to it. The line has 80 columns, and the root
/// would take most of them.
fn relative<'a>(path: &'a str, root: &str) -> &'a str {
    match path.strip_prefix(root).and_then(|p| p.strip_prefix('/')) {
        Some(rest) if !root.is_empty() && !rest.is_empty() => rest,
        _ => path,
    }
}

/// Models often open every command with `cd <root> &&`, though `bash` already runs there. Shown,
/// it fills the cut line and hides the command. Only the display drops it.
fn without_cd<'a>(command: &'a str, root: &str) -> &'a str {
    let Some(rest) = command.trim_start().strip_prefix("cd ") else {
        return command;
    };
    let rest = rest.trim_start();
    for dir in [format!("'{root}'"), format!("\"{root}\""), root.to_string()] {
        if let Some(after) = rest.strip_prefix(dir.as_str()) {
            let after = after.trim_start();
            if let Some(tail) = after.strip_prefix("&&").or_else(|| after.strip_prefix(';')) {
                return tail.trim_start();
            }
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::{describe, one_line, printable};

    /// The root is what a model repeats and what the cut line cannot spare.
    #[test]
    fn the_working_directory_is_left_out_of_the_line() {
        let root = std::env::current_dir().unwrap().display().to_string();
        let d = |name, args: serde_json::Value| describe(name, &args.to_string(), 80);
        assert_eq!(
            d(
                "bash",
                serde_json::json!({"command": format!("cd {root} && ls -la")})
            ),
            "[tool] bash ls -la"
        );
        assert_eq!(
            d(
                "bash",
                serde_json::json!({"command": format!("cd '{root}'; make")})
            ),
            "[tool] bash make"
        );
        assert_eq!(
            d(
                "bash",
                serde_json::json!({"command": "cd /elsewhere && ls"})
            ),
            "[tool] bash cd /elsewhere && ls"
        );
        assert_eq!(
            d(
                "write",
                serde_json::json!({"path": format!("{root}/docs/REVIEW.md")})
            ),
            "[tool] write docs/REVIEW.md"
        );
        assert_eq!(
            d("read", serde_json::json!({"path": format!("{root}x/a")})),
            format!("[tool] read {root}x/a")
        );
    }

    #[test]
    fn describes_known_calls_by_their_target() {
        let d = |name, args| describe(name, args, 80);
        assert_eq!(
            d("bash", r#"{"command":"cargo test","timeout_ms":1}"#),
            "[tool] bash cargo test"
        );
        assert_eq!(
            d("read", r#"{"path":"a.rs","offset":495,"limit":500}"#),
            "[tool] read a.rs:495-994"
        );
        assert_eq!(
            d("read", r#"{"path":"a.rs","offset":7}"#),
            "[tool] read a.rs:7-"
        );
        assert_eq!(d("read", r#"{"path":"a.rs"}"#), "[tool] read a.rs");
        assert_eq!(
            d("edit", r#"{"path":"a.rs","old":"x","new":"y"}"#),
            "[tool] edit a.rs"
        );
        // Anything unrecognised is shown as it arrived.
        assert_eq!(d("bash", "{not json"), "[tool] bash {not json");
        assert_eq!(d("grep", r#"{"q":1}"#), r#"[tool] grep {"q":1}"#);
    }

    #[test]
    fn printable_strips_escapes_but_keeps_layout() {
        assert_eq!(
            printable("a\x1b]52;c;aGk=\x07b\r\n\tc\u{9b}d"),
            "a]52;c;aGk=b\n\tcd"
        );
    }

    #[test]
    fn collapses_control_characters() {
        assert_eq!(one_line("a\nb\tc", 40), "a b c");
    }

    #[test]
    fn truncates_on_character_boundaries() {
        // 5 characters into a width of 4: one kept, then the ellipsis.
        assert_eq!(
            one_line("\u{00e9}\u{00e9}\u{00e9}\u{00e9}\u{00e9}", 4),
            "\u{00e9}..."
        );
        // Exactly at the width is not truncated.
        assert_eq!(
            one_line("\u{00e9}\u{00e9}\u{00e9}\u{00e9}", 4),
            "\u{00e9}\u{00e9}\u{00e9}\u{00e9}"
        );
    }

    #[test]
    fn leaves_short_text_alone() {
        assert_eq!(one_line("  hi  ", 10), "hi");
    }
}
