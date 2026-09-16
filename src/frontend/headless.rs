//! `-p`: assistant text on stdout, everything else on stderr, so the output pipes cleanly.

use std::io::Write;
use std::time::Duration;

use super::{Frontend, one_line};
use crate::provider::Usage;
use crate::theme::{self, Style};

pub struct Headless {
    quiet: bool,
    /// stdout and stderr are separate streams, so a note written while assistant text sits
    /// mid-line runs the two together on a terminal. Tracked here exactly as the REPL does it.
    line_open: bool,
}

impl Headless {
    pub fn new(quiet: bool) -> Self {
        Self {
            quiet,
            line_open: false,
        }
    }

    fn note(&mut self, style: Style, text: &str) {
        if self.quiet {
            return;
        }
        if self.line_open {
            println!();
            self.line_open = false;
            let _ = std::io::stdout().flush();
        }
        eprintln!("{}", theme::paint(style, text));
    }
}

impl Frontend for Headless {
    fn text(&mut self, delta: &str) {
        print!("{delta}");
        self.line_open = !delta.ends_with('\n');
        let _ = std::io::stdout().flush();
    }

    fn tool_start(&mut self, name: &str, arguments: &str) {
        self.note(
            Style::Muted,
            &format!("[{name}] {}", one_line(arguments, 80)),
        );
    }

    fn tool_end(&mut self, _name: &str, body: &str, note: Option<&str>, ok: bool) {
        if !ok {
            self.note(Style::Error, &format!("  -> {}", one_line(body, 80)));
        } else if let Some(note) = note {
            self.note(Style::Warn, &format!("  -> {note}"));
        }
    }

    fn retry(&mut self, attempt: u32, delay: Duration) {
        let text = format!("retrying ({attempt}) in {:.1}s", delay.as_secs_f32());
        self.note(Style::Warn, &text);
    }

    fn turn_end(&mut self, _usage: Usage) {}

    fn cancelled(&mut self) {
        self.note(Style::Warn, "cancelled");
    }
}
