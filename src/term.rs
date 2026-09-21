//! Terminal mode ownership.
//!
//! The REPL runs in raw mode with bracketed paste on. Both are undone by `restore`, from the
//! screen's `Drop`, a panic hook and the signal handler. Losing it leaves the user's shell in raw
//! mode after a crash, which is why `profile.release` keeps `panic = "unwind"`.

use crossterm::{cursor, event, execute, terminal};

/// A no-op unless the REPL entered raw mode, so `-p` output piped elsewhere gets no escapes.
pub fn restore() {
    if !terminal::is_raw_mode_enabled().unwrap_or(false) {
        return;
    }
    let _ = execute!(
        std::io::stdout(),
        event::DisableBracketedPaste,
        cursor::Show
    );
    let _ = terminal::disable_raw_mode();
}

/// Restore the terminal before the default hook prints, so the panic message is readable.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}
