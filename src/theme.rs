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
    let on = !disabled_by_flag
        && std::env::var_os("NO_COLOR").is_none()
        && std::io::stdout().is_terminal();
    ENABLED.store(on, Ordering::Relaxed);
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
    if enabled() {
        format!("{}{text}{RESET}", style.code())
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests share a process, so drive the flag directly rather than through init().
    fn with_colour<T>(on: bool, f: impl FnOnce() -> T) -> T {
        let previous = ENABLED.swap(on, Ordering::Relaxed);
        let out = f();
        ENABLED.store(previous, Ordering::Relaxed);
        out
    }

    #[test]
    fn colour_off_leaves_text_untouched() {
        with_colour(false, || {
            assert_eq!(paint(Style::Error, "boom"), "boom");
        });
    }

    #[test]
    fn colour_on_wraps_and_always_resets() {
        with_colour(true, || {
            let painted = paint(Style::Muted, "hi");
            assert!(painted.starts_with("\x1b[2m"));
            assert!(painted.ends_with(RESET));
            assert!(painted.contains("hi"));
        });
    }

    #[test]
    fn the_flag_beats_a_terminal() {
        init(true);
        assert!(!enabled());
    }
}
