//! Colour, decided once at startup.
//!
//! Three inputs, in order: `--no-color`, the `NO_COLOR` convention, and whether the stream is a
//! terminal. A pipe gets no escapes, so redirected output stays diffable.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// `stdout` decides, because that is where assistant text goes; notices follow it so both halves
/// of a run agree.
pub fn init(disabled_by_flag: bool) {
    let on = decide(
        disabled_by_flag,
        std::env::var_os("NO_COLOR").is_some(),
        std::io::stdout().is_terminal(),
    );
    ENABLED.store(on, Ordering::Relaxed);
}

fn decide(disabled_by_flag: bool, no_color: bool, terminal: bool) -> bool {
    !disabled_by_flag && !no_color && terminal
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// A semantic role, not a colour name, so the palette can change in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Tool calls, token counts, anything secondary to the answer.
    Muted,
    /// A tool that ran but reported a problem.
    Warn,
    /// A failure.
    Error,
}

impl Style {
    fn code(self) -> &'static str {
        match self {
            Style::Muted => "\x1b[2m",
            Style::Warn => "\x1b[33m",
            Style::Error => "\x1b[31m",
        }
    }
}

const RESET: &str = "\x1b[0m";

/// Wraps `text` when colour is on, and returns it untouched when it is off.
pub fn paint(style: Style, text: &str) -> String {
    paint_if(enabled(), style, text)
}

/// Takes the switch as an argument, so tests need not write the process-wide flag.
fn paint_if(on: bool, style: Style, text: &str) -> String {
    if on {
        format!("{}{text}{RESET}", style.code())
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_off_leaves_text_untouched() {
        assert_eq!(paint_if(false, Style::Error, "boom"), "boom");
    }

    #[test]
    fn colour_on_wraps_and_always_resets() {
        let painted = paint_if(true, Style::Muted, "hi");
        assert!(painted.starts_with("\x1b[2m"));
        assert!(painted.ends_with(RESET));
        assert!(painted.contains("hi"));
    }

    #[test]
    fn a_terminal_gets_colour_unless_the_flag_or_no_color_says_otherwise() {
        assert!(decide(false, false, true));
        assert!(!decide(true, false, true));
        assert!(!decide(false, true, true));
        assert!(!decide(false, false, false));
    }
}
