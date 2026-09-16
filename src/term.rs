//! Terminal mode ownership.
//!
//! Raw mode is restored from `Drop` and from a panic hook. Losing either leaves the user's shell
//! in raw mode after a crash, which is why `profile.release` keeps `panic = "unwind"`.

use anyhow::Result;
use crossterm::terminal;

pub struct RawGuard {
    armed: bool,
}

impl RawGuard {
    pub fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self { armed: true })
    }

    /// Leave raw mode early, for a subprocess that wants the tty in cooked mode.
    pub fn release(&mut self) {
        if self.armed {
            let _ = terminal::disable_raw_mode();
            self.armed = false;
        }
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        self.release();
    }
}

/// Restore cooked mode before the default hook prints, so the panic message is readable.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        previous(info);
    }));
}
