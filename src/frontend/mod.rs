//! Presentation, kept out of the agent loop.
//!
//! Sync on purpose: every method is a write to an already-open stream, so making the trait async
//! would buy nothing and cost object safety.

pub mod headless;
mod history;
pub mod json;
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

    fn cancelled(&mut self);
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

/// A tool call as a user would type it, `read src/lib.rs:495-994` rather than its JSON. Unknown
/// tools and arguments that do not parse fall back to the raw form.
pub fn describe(name: &str, arguments: &str, width: usize) -> String {
    let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let path = args["path"].as_str();
    let text = match (name, path, args["command"].as_str()) {
        ("bash", _, Some(command)) => format!("$ {command}"),
        ("read", Some(path), _) => {
            let start = args["offset"].as_u64().unwrap_or(1).max(1);
            match args["limit"].as_u64() {
                Some(limit) => format!("read {path}:{start}-{}", start + limit.max(1) - 1),
                None if start > 1 => format!("read {path}:{start}-"),
                None => format!("read {path}"),
            }
        }
        ("write" | "edit", Some(path), _) => format!("{name} {path}"),
        _ => format!("{name}({arguments})"),
    };
    one_line(&text, width)
}

#[cfg(test)]
mod tests {
    use super::{describe, one_line, printable};

    #[test]
    fn describes_known_calls_by_their_target() {
        let d = |name, args| describe(name, args, 80);
        assert_eq!(
            d("bash", r#"{"command":"cargo test","timeout_ms":1}"#),
            "$ cargo test"
        );
        assert_eq!(
            d("read", r#"{"path":"a.rs","offset":495,"limit":500}"#),
            "read a.rs:495-994"
        );
        assert_eq!(d("read", r#"{"path":"a.rs","offset":7}"#), "read a.rs:7-");
        assert_eq!(d("read", r#"{"path":"a.rs"}"#), "read a.rs");
        assert_eq!(
            d("edit", r#"{"path":"a.rs","old":"x","new":"y"}"#),
            "edit a.rs"
        );
        // Anything unrecognised is shown as it arrived.
        assert_eq!(d("bash", "{not json"), "bash({not json)");
        assert_eq!(d("grep", r#"{"q":1}"#), r#"grep({"q":1})"#);
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
