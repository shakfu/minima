//! The interactive frontend, and the only place that owns the terminal.
//!
//! reedline reads the line in cooked mode. minima then enters raw mode for the duration of the
//! turn so a watcher thread can see Esc and Ctrl-C, which means every write has to carry its own carriage
//! return.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};

use super::{Frontend, describe, one_line, printable};
use crate::agent::Agent;
use crate::cancel::Cancel;
use crate::provider::Usage;
use crate::term::RawGuard;
use crate::theme::{self, Style};

const POLL: Duration = Duration::from_millis(80);
const HISTORY_CAPACITY: usize = 1000;

/// Tool lines and the usage summary stay within this many columns.
const WIDTH: usize = 80;
const CALL_WIDTH: usize = 56;

#[derive(Default)]
pub struct Repl {
    line_open: bool,
    /// Columns the open tool line already takes.
    call_width: usize,
    context: u32,
    /// Tokens the last round-trip reported: what the next request will carry.
    used: u32,
    /// Summed over the round-trips of one prompt.
    input: u64,
    output: u64,
    cost: Option<f64>,
    session_cost: Option<f64>,
    /// Costs come from a price list, not the provider, and are marked `~`.
    estimate: bool,
}

impl Repl {
    /// Raw mode swallows the carriage return, so `\n` has to be rewritten on the way out.
    fn put(&mut self, text: &str) {
        let out = text.replace('\n', "\r\n");
        print!("{out}");
        self.line_open = !out.ends_with('\n');
        let _ = std::io::stdout().flush();
    }

    fn line(&mut self, text: &str) {
        if self.line_open {
            self.put("\n");
        }
        self.put(&format!("{text}\n"));
    }

    /// One line per prompt, after its last round-trip, then the counters restart.
    fn finish(&mut self) {
        if self.input + self.output > 0 {
            let text = usage_line(
                self.used,
                self.context,
                self.input,
                self.output,
                self.cost,
                self.session_cost,
                self.estimate,
            );
            self.line(&theme::paint(Style::Muted, &text));
        }
        self.used = 0;
        self.input = 0;
        self.output = 0;
        self.cost = None;
    }
}

/// `39.3k/1.05M context, 312.4k in, 8.1k out, $0.0123 (session $0.0456)`. Cost only when known,
/// and the session total only once it differs from this prompt's.
fn usage_line(
    used: u32,
    context: u32,
    input: u64,
    output: u64,
    cost: Option<f64>,
    session: Option<f64>,
    estimate: bool,
) -> String {
    let dollars = |usd: f64| format!("{}{}", if estimate { "~" } else { "" }, dollars(usd));
    let mut text = format!(
        "{}/{} context, {} in, {} out",
        count(used.into()),
        count(context.into()),
        count(input),
        count(output)
    );
    if let Some(cost) = cost {
        text.push_str(&format!(", {}", dollars(cost)));
        if let Some(session) = session.filter(|s| s - cost > 1e-9) {
            text.push_str(&format!(" (session {})", dollars(session)));
        }
    }
    text
}

/// A one-line result is shown whole; anything longer by its size, not counting `read`'s
/// `... N more lines` footers.
fn result(body: &str) -> String {
    match body
        .trim()
        .lines()
        .filter(|line| !line.starts_with("... "))
        .count()
    {
        0 => "no output".to_string(),
        1 => body.trim().to_string(),
        n => format!("{n} lines"),
    }
}

fn count(n: u64) -> String {
    let (value, unit, places) = match n {
        0..1_000 => return n.to_string(),
        1_000..1_000_000 => (n as f64 / 1e3, "k", 1),
        _ => (n as f64 / 1e6, "M", 2),
    };
    let text = format!("{value:.places$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}{unit}")
}

fn dollars(usd: f64) -> String {
    if usd < 1.0 {
        format!("${usd:.4}")
    } else {
        format!("${usd:.2}")
    }
}

impl Frontend for Repl {
    fn text(&mut self, delta: &str) {
        self.put(&printable(delta));
    }

    /// Left open until `tool_end` adds the status, so each call takes one line.
    fn tool_start(&mut self, name: &str, arguments: &str) {
        if self.line_open {
            self.put("\n");
        }
        let text = describe(name, arguments, CALL_WIDTH);
        self.put(&theme::paint(Style::Muted, &text));
        self.call_width = text.chars().count();
    }

    fn tool_end(&mut self, body: &str, note: Option<&str>, ok: bool) {
        // A note displaces the result because it says more: it is the reason the model is about
        // to try something else.
        let room = WIDTH.saturating_sub(self.call_width + 4).max(16);
        let status = |text: &str| format!(" -> {}", one_line(text, room));
        let text = match (ok, note) {
            (false, _) => theme::paint(Style::Error, &status(body)),
            (true, Some(note)) => theme::paint(Style::Warn, &status(note)),
            (true, None) => theme::paint(Style::Muted, &status(&result(body))),
        };
        self.put(&format!("{text}\n"));
    }

    fn retry(&mut self, attempt: u32, delay: Duration) {
        let text = format!("retrying ({attempt}) in {:.1}s", delay.as_secs_f32());
        self.line(&theme::paint(Style::Warn, &text));
    }

    fn turn_end(&mut self, usage: Usage) {
        if usage.total_tokens > 0 {
            self.used = usage.total_tokens;
        }
        self.input += u64::from(usage.prompt_tokens);
        self.output += u64::from(usage.completion_tokens);
        if let Some(cost) = usage.cost {
            self.cost = Some(self.cost.unwrap_or(0.0) + cost);
            self.session_cost = Some(self.session_cost.unwrap_or(0.0) + cost);
        }
    }

    fn cancelled(&mut self) {
        self.line(&theme::paint(Style::Warn, "cancelled"));
    }
}

/// Prompt history survives the session. It cannot be recovered if the file is unusable, so a
/// failure degrades to the in-memory default rather than refusing to start.
fn editor_with_history() -> Reedline {
    let Some(path) = crate::config::history_path() else {
        return Reedline::create();
    };
    match FileBackedHistory::with_file(HISTORY_CAPACITY, path.clone()) {
        Ok(history) => {
            // Prompts are written verbatim, so the file and the directory holding it are
            // owner-only. with_file() has already created both.
            for target in [path.as_path(), path.parent().unwrap_or(&path)] {
                if let Err(e) = crate::config::restrict_to_owner(target) {
                    tracing::warn!("could not restrict {}: {e}", target.display());
                }
            }
            // Keeps /quit out of the recall ring; otherwise the first Up in a fresh session hands
            // the user the exit command. Only /quit: a prompt can start with a path.
            Reedline::create()
                .with_history(Box::new(history))
                .with_history_exclusion_prefix(Some("/quit".into()))
        }
        Err(e) => {
            tracing::warn!("history disabled, staying in memory: {e}");
            Reedline::create().with_history_exclusion_prefix(Some("/quit".into()))
        }
    }
}

/// Runs on the main thread and drives the runtime with `block_on`, so reedline's blocking read
/// never sits inside an async task.
pub fn run(runtime: &tokio::runtime::Runtime, agent: &mut Agent) -> Result<()> {
    let mut editor = editor_with_history();
    let prompt = DefaultPrompt::default();
    let cancel = Cancel::new();
    let mut frontend = Repl {
        context: agent.context_window(),
        estimate: agent.cost_is_estimate(),
        ..Repl::default()
    };

    loop {
        let line = match editor.read_line(&prompt)? {
            Signal::Success(line) => line,
            Signal::CtrlC => continue,
            _ => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "/quit" {
            break;
        }

        let guard = RawGuard::enter()?;
        let stop = Arc::new(AtomicBool::new(false));
        let watcher = watch_for_esc(cancel.clone(), Arc::clone(&stop));

        let result = runtime.block_on(agent.run(trimmed, &mut frontend, &cancel));

        stop.store(true, Ordering::SeqCst);
        let _ = watcher.join();
        frontend.finish();
        frontend.line("");
        drop(guard);

        if let Err(e) = result {
            let text = printable(&format!("error: {e:#}"));
            eprintln!("{}", theme::paint(Style::Error, &text));
        }
    }
    Ok(())
}

/// Raw mode turns Ctrl-C into a key event rather than SIGINT, so it is matched here beside Esc.
///
/// A thread, not a task: `crossterm::event::read` blocks, and crossterm's async event stream
/// would be another feature to carry for one key.
fn watch_for_esc(cancel: Cancel, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match event::poll(POLL) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(_) => break,
            }
            if let Ok(event::Event::Key(key)) = event::read()
                && key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)))
            {
                cancel.cancel();
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{count, result, usage_line};

    #[test]
    fn counts_are_short() {
        assert_eq!(count(508), "508");
        assert_eq!(count(39_317), "39.3k");
        assert_eq!(count(400_000), "400k");
        assert_eq!(count(1_050_000), "1.05M");
        assert_eq!(count(1_000_000), "1M");
    }

    #[test]
    fn the_usage_line_adds_cost_only_when_reported() {
        assert_eq!(
            usage_line(39_317, 1_050_000, 312_400, 8_100, None, None, false),
            "39.3k/1.05M context, 312.4k in, 8.1k out"
        );
        assert_eq!(
            usage_line(10, 1000, 10, 2, Some(0.0123), Some(0.0123), false),
            "10/1k context, 10 in, 2 out, $0.0123"
        );
        assert_eq!(
            usage_line(10, 1000, 10, 2, Some(0.0123), Some(1.5), false),
            "10/1k context, 10 in, 2 out, $0.0123 (session $1.50)"
        );
        assert_eq!(
            usage_line(10, 1000, 10, 2, Some(0.0123), Some(1.5), true),
            "10/1k context, 10 in, 2 out, ~$0.0123 (session ~$1.50)"
        );
    }

    #[test]
    fn a_result_is_shown_whole_only_when_it_is_one_line() {
        assert_eq!(
            result("wrote 6049 bytes to REVIEW.md\n"),
            "wrote 6049 bytes to REVIEW.md"
        );
        assert_eq!(result("1 a\n2 b\n3 c\n"), "3 lines");
        assert_eq!(result("1 a\n2 b\n... 57 more lines\n"), "2 lines");
        assert_eq!(result("  \n"), "no output");
    }
}
