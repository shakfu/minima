//! Presentation, kept out of the agent loop.
//!
//! Sync on purpose: every method is a write to an already-open stream, so making the trait async
//! would buy nothing and cost object safety.

pub mod headless;
pub mod repl;

use std::time::Duration;

use crate::provider::Usage;

pub trait Frontend {
    /// A fragment of assistant text, as it streams.
    fn text(&mut self, delta: &str);

    fn tool_start(&mut self, name: &str, arguments: &str);

    /// `note` is set when the tool ran but reported a problem, such as a non-zero exit. `ok` is
    /// false only when the tool itself failed.
    fn tool_end(&mut self, name: &str, body: &str, note: Option<&str>, ok: bool);

    fn retry(&mut self, attempt: u32, delay: Duration);

    fn turn_end(&mut self, usage: Usage);

    fn cancelled(&mut self);
}

/// Tool arguments and results are echoed as one line. The full text is in the transcript the
/// model sees; the frontend only has to show that something ran.
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

#[cfg(test)]
mod tests {
    use super::one_line;

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
