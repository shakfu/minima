//! The interactive frontend, and the only place that owns the terminal.
//!
//! Two threads share the `Screen`. A keyboard thread owns input for the whole session: it edits
//! and submits while idle, and during a turn it cancels on Esc or Ctrl-C, keeps accepting
//! type-ahead and redraws the spinner. The main thread runs the agent and writes its output.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::history::History;
use super::screen::{Screen, Tone};
use super::{Frontend, describe, one_line, printable};
use crate::agent::Agent;
use crate::cancel::Cancel;
use crate::provider::Usage;
use crate::theme::Style;

/// How often the keyboard thread wakes to check for a stop and to move the spinner.
const POLL: Duration = Duration::from_millis(120);

/// Tool lines stay within this many columns.
const WIDTH: usize = 80;
const CALL_WIDTH: usize = 56;

pub const EXIT: [&str; 2] = ["/quit", "/exit"];

/// Token and cost counters, apart from the terminal so they can be tested without one.
#[derive(Default)]
struct Meter {
    context: u32,
    /// Tokens the last round-trip reported: what the next request will carry. Not reset between
    /// prompts, because the conversation carries over.
    used: u32,
    /// Summed over the round-trips of one prompt.
    input: u64,
    output: u64,
    cost: Option<f64>,
    session_cost: Option<f64>,
    /// Costs come from a price list, not the provider, and are marked `~`.
    estimate: bool,
}

impl Meter {
    fn add(&mut self, usage: Usage) {
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

    /// The prompt's usage line, if it used anything, and a fresh count for the next prompt.
    fn finish(&mut self) -> Option<String> {
        let line = (self.input + self.output > 0).then(|| {
            usage_line(
                self.used,
                self.context,
                self.input,
                self.output,
                self.cost,
                self.session_cost,
                self.estimate,
            )
        });
        (self.input, self.output, self.cost) = (0, 0, None);
        line
    }

    /// The right side of the status bar: the model, then context used and the session's cost
    /// once known.
    fn status(&self, model: &str) -> String {
        let mut text = model.to_string();
        if self.used > 0 {
            let (used, window) = (count(self.used.into()), count(self.context.into()));
            text.push_str(&format!("  {used}/{window}"));
        }
        if let Some(cost) = self.session_cost {
            text.push_str(&format!("  {}", dollars(cost, self.estimate)));
        }
        text
    }
}

pub struct Repl {
    screen: Arc<Mutex<Screen>>,
    meter: Meter,
    /// The running call as `describe` renders it, until its result completes the line.
    call: Option<String>,
    /// The last row written was a tool line, so the answer that follows needs a gap.
    after_tool: bool,
}

impl Repl {
    fn screen(&self) -> MutexGuard<'_, Screen> {
        lock(&self.screen)
    }
}

fn lock(screen: &Mutex<Screen>) -> MutexGuard<'_, Screen> {
    screen.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Frontend for Repl {
    fn text(&mut self, delta: &str) {
        let delta = printable(delta);
        let gap = std::mem::take(&mut self.after_tool) && !delta.is_empty();
        let mut screen = self.screen();
        if gap {
            let _ = screen.line(Tone::Plain, "");
        }
        let _ = screen.text(&delta);
        let _ = screen.draw();
    }

    /// Shown in the viewport while it runs, and committed with its result.
    fn tool_start(&mut self, name: &str, arguments: &str) {
        let call = describe(name, arguments, CALL_WIDTH);
        let mut screen = self.screen();
        let _ = screen.flush();
        screen.set_activity(Some(format!("{call} ...")));
        let _ = screen.draw();
        drop(screen);
        self.call = Some(call);
    }

    fn tool_end(&mut self, body: &str, note: Option<&str>, ok: bool) {
        let call = self.call.take().unwrap_or_default();
        let room = WIDTH.saturating_sub(call.chars().count() + 4).max(16);
        let (tone, status) = tool_status(body, note, ok, room);
        let mut screen = self.screen();
        screen.set_activity(None);
        let _ = screen.spans(vec![
            (Tone::Role(Style::Muted), call),
            (Tone::Role(tone), format!(" -> {status}")),
        ]);
        let _ = screen.draw();
        drop(screen);
        self.after_tool = true;
    }

    fn retry(&mut self, attempt: u32, delay: Duration) {
        let text = format!("retrying ({attempt}) in {:.1}s", delay.as_secs_f32());
        let mut screen = self.screen();
        let _ = screen.line(Tone::Role(Style::Warn), &text);
        let _ = screen.draw();
    }

    fn turn_end(&mut self, usage: Usage) {
        self.meter.add(usage);
    }

    fn cancelled(&mut self) {
        let mut screen = self.screen();
        screen.set_activity(None);
        let _ = screen.line(Tone::Role(Style::Warn), "cancelled");
        let _ = screen.draw();
    }
}

enum Request {
    Prompt(String),
    Exit,
}

/// Runs on the main thread and drives the runtime with `block_on`; the keyboard thread hands
/// over each submitted prompt.
pub fn run(runtime: &tokio::runtime::Runtime, agent: &mut Agent) -> Result<()> {
    let history = History::load(crate::config::history_path(), &EXIT);
    let screen = Arc::new(Mutex::new(Screen::new()?));
    let cancel = Cancel::new();
    let busy = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    let mut frontend = Repl {
        screen: Arc::clone(&screen),
        meter: Meter {
            context: agent.context_window(),
            estimate: agent.cost_is_estimate(),
            ..Meter::default()
        },
        call: None,
        after_tool: false,
    };
    {
        let mut screen = frontend.screen();
        screen.line(Tone::Title, concat!("minima ", env!("CARGO_PKG_VERSION")))?;
        screen.status = frontend.meter.status(agent.model());
        screen.draw()?;
    }

    let (tx, rx) = mpsc::channel();
    let keyboard = std::thread::spawn({
        let keys = Keyboard {
            screen: Arc::clone(&screen),
            cancel: cancel.clone(),
            busy: Arc::clone(&busy),
            stop: Arc::clone(&stop),
            history,
            search: None,
            tx,
        };
        move || keys.run()
    });

    while let Ok(Request::Prompt(prompt)) = rx.recv() {
        let result = runtime.block_on(agent.run(&prompt, &mut frontend, &cancel));
        frontend.after_tool = false;
        let usage = frontend.meter.finish();
        let status = frontend.meter.status(agent.model());
        let mut screen = frontend.screen();
        screen.set_activity(None);
        if let Err(e) = result {
            let _ = screen.line(
                Tone::Role(Style::Error),
                &printable(&format!("error: {e:#}")),
            );
        }
        if let Some(usage) = usage {
            let _ = screen.line(Tone::Role(Style::Muted), &usage);
        }
        screen.status = status;
        screen.busy_since = None;
        busy.store(false, Ordering::SeqCst);
        let _ = screen.draw();
    }

    stop.store(true, Ordering::SeqCst);
    let _ = keyboard.join();
    lock(&screen).close()
}

struct Keyboard {
    screen: Arc<Mutex<Screen>>,
    cancel: Cancel,
    busy: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    history: History,
    search: Option<Search>,
    tx: Sender<Request>,
}

/// Ctrl-R, as in readline: each key refines the query, Ctrl-R steps to an older match, Enter or
/// any editing key accepts it, and Esc or Ctrl-G restores what was typed before.
struct Search {
    query: String,
    found: Option<usize>,
    /// Nothing older matches the query; the last match stays shown.
    failed: bool,
    saved: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    Update,
    Cancel,
    Accept,
    /// Accept, then handle the key as an ordinary edit.
    AcceptAndPass,
}

impl Search {
    fn new(saved: String) -> Self {
        Self {
            query: String::new(),
            found: None,
            failed: false,
            saved,
        }
    }

    fn key(&mut self, key: KeyEvent, history: &History) -> Step {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char('r') if ctrl => self.find(history, self.found.unwrap_or(history.len())),
            KeyCode::Char('g') if ctrl => return Step::Cancel,
            KeyCode::Esc => return Step::Cancel,
            KeyCode::Enter => return Step::Accept,
            KeyCode::Backspace => {
                self.query.pop();
                self.find(history, history.len());
            }
            // A longer query can still match the entry shown, so the search starts there.
            KeyCode::Char(c) if !ctrl && !alt => {
                self.query.push(c);
                self.find(history, self.found.map_or(history.len(), |i| i + 1));
            }
            _ => return Step::AcceptAndPass,
        }
        Step::Update
    }

    fn find(&mut self, history: &History, before: usize) {
        match history.search(&self.query, before) {
            Some(i) => (self.found, self.failed) = (Some(i), false),
            None => self.failed = true,
        }
    }

    fn label(&self) -> String {
        let failed = if self.failed { "failed " } else { "" };
        format!("({failed}reverse-i-search)`{}': ", self.query)
    }
}

impl Keyboard {
    /// A thread, not a task: `crossterm::event::read` blocks, and crossterm's async event stream
    /// would be another feature to carry for one loop.
    fn run(mut self) {
        while !self.stop.load(Ordering::SeqCst) {
            let ready = match event::poll(POLL) {
                Ok(ready) => ready,
                Err(_) => break,
            };
            if ready {
                match event::read() {
                    Ok(event) => {
                        if !self.handle(event) {
                            let _ = self.tx.send(Request::Exit);
                            return;
                        }
                    }
                    Err(_) => break,
                }
            }
            // Also on a timeout, so the spinner turns while a turn runs.
            let _ = lock(&self.screen).draw();
        }
        let _ = self.tx.send(Request::Exit);
    }

    /// False when the user asked to leave.
    fn handle(&mut self, event: Event) -> bool {
        let busy = self.busy.load(Ordering::SeqCst);
        let key = match event {
            Event::Paste(text) => {
                lock(&self.screen)
                    .input
                    .insert_str(text.replace('\r', "\n"));
                return true;
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => key,
            _ => return true,
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        // Raw mode turns Ctrl-C into a key event rather than SIGINT, so it is matched beside Esc.
        let interrupt = key.code == KeyCode::Esc || (ctrl && key.code == KeyCode::Char('c'));
        if busy && interrupt {
            self.cancel.cancel();
            return true;
        }
        let mut screen = lock(&self.screen);
        if let Some(search) = &mut self.search {
            let step = search.key(key, &self.history);
            match step {
                Step::Update => {
                    if let Some(i) = search.found {
                        screen.set_input(self.history.get(i));
                    }
                    screen.search = Some(search.label());
                    return true;
                }
                Step::Cancel => screen.set_input(&search.saved),
                Step::Accept | Step::AcceptAndPass => {}
            }
            screen.search = None;
            self.search = None;
            if step != Step::AcceptAndPass {
                return true;
            }
        }
        // Ctrl-P and Ctrl-N browse history as readline's do, once the cursor is on the edge row.
        let older = key.code == KeyCode::Up || (ctrl && key.code == KeyCode::Char('p'));
        let newer = key.code == KeyCode::Down || (ctrl && key.code == KeyCode::Char('n'));
        match key.code {
            KeyCode::Char('r') if ctrl => {
                let search = Search::new(screen.input_text());
                screen.search = Some(search.label());
                self.search = Some(search);
            }
            KeyCode::Char('c') if ctrl => screen.set_input(""),
            KeyCode::Char('d') if ctrl && screen.input.is_empty() => return false,
            KeyCode::Enter if alt => screen.input.insert_newline(),
            KeyCode::Char('j') if ctrl => screen.input.insert_newline(),
            KeyCode::Enter if busy => {}
            KeyCode::Enter => {
                let prompt = screen.input_text();
                let trimmed = prompt.trim();
                if EXIT.contains(&trimmed) {
                    return false;
                }
                if trimmed.is_empty() {
                    return true;
                }
                self.history.push(trimmed);
                screen.set_input("");
                let _ = screen.line(Tone::Plain, "");
                let _ = screen.line(Tone::Prompt, &format!("> {trimmed}"));
                screen.busy_since = Some(Instant::now());
                self.busy.store(true, Ordering::SeqCst);
                let _ = self.tx.send(Request::Prompt(trimmed.to_string()));
            }
            _ if older && screen.input.cursor().0 == 0 => {
                if let Some(entry) = self.history.prev(&screen.input_text()) {
                    screen.set_input(&entry);
                }
            }
            _ if newer && screen.input.cursor().0 + 1 >= screen.input.lines().len() => {
                if let Some(entry) = self.history.next() {
                    screen.set_input(&entry);
                }
            }
            _ => {
                screen.input.input(key);
            }
        }
        true
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
    let dollars = |usd: f64| dollars(usd, estimate);
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

/// What follows a finished call's `->`. A note displaces the result because it says more: it is
/// the reason the model is about to try something else. So a note or an error is flattened but
/// never cut, and wraps if it must; only a routine result is cut to `room` columns.
fn tool_status(body: &str, note: Option<&str>, ok: bool, room: usize) -> (Style, String) {
    match (ok, note) {
        (false, note) => (Style::Error, one_line(note.unwrap_or(body), usize::MAX)),
        (true, Some(note)) => (Style::Warn, one_line(note, usize::MAX)),
        (true, None) => (Style::Muted, one_line(&result(body), room)),
    }
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

/// An estimate is marked `~`.
fn dollars(usd: f64, estimate: bool) -> String {
    let mark = if estimate { "~" } else { "" };
    if usd < 1.0 {
        format!("{mark}${usd:.4}")
    } else {
        format!("{mark}${usd:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::{Meter, Search, Step, count, result, tool_status, usage_line};
    use crate::theme::Style;

    /// A hint after a long stderr line, such as `--confine fs`'s note on a denied write, must
    /// reach the user; the routine result is what gets cut.
    #[test]
    fn notes_and_errors_are_never_cut() {
        let note = format!(
            "exit 1: {}; if a write was denied: see --writable",
            "x".repeat(60)
        );
        let (tone, shown) = tool_status("", Some(&note), true, 20);
        assert_eq!((tone, shown.as_str()), (Style::Warn, note.as_str()));
        let (tone, shown) = tool_status("", Some("a\nlong\nerror"), false, 5);
        assert_eq!((tone, shown.as_str()), (Style::Error, "a long error"));
        let (_, shown) = tool_status(&"y".repeat(60), None, true, 20);
        assert_eq!(shown.chars().count(), 20);
    }
    use crate::frontend::history::History;
    use crate::provider::Usage;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn history() -> History {
        let mut h = History::load(None, &[]);
        for entry in ["cargo test", "fix the bug", "cargo build", "review it"] {
            h.push(entry);
        }
        h
    }

    fn typed(search: &mut Search, h: &History, text: &str) {
        for c in text.chars() {
            assert_eq!(search.key(key(KeyCode::Char(c)), h), Step::Update);
        }
    }

    #[test]
    fn ctrl_r_finds_the_newest_match_then_older_ones() {
        let h = history();
        let mut s = Search::new("draft".into());
        typed(&mut s, &h, "cargo");
        assert_eq!(s.found.map(|i| h.get(i)), Some("cargo build"));
        s.key(ctrl('r'), &h);
        assert_eq!(s.found.map(|i| h.get(i)), Some("cargo test"));
        // Nothing older: the last match stays and the prompt says so.
        s.key(ctrl('r'), &h);
        assert_eq!(s.found.map(|i| h.get(i)), Some("cargo test"));
        assert_eq!(s.label(), "(failed reverse-i-search)`cargo': ");
    }

    #[test]
    fn a_longer_query_keeps_the_match_it_still_fits_and_backspace_widens_again() {
        let h = history();
        let mut s = Search::new(String::new());
        typed(&mut s, &h, "cargo t");
        assert_eq!(s.found.map(|i| h.get(i)), Some("cargo test"));
        s.key(key(KeyCode::Backspace), &h);
        s.key(key(KeyCode::Backspace), &h);
        assert_eq!(s.found.map(|i| h.get(i)), Some("cargo build"));
    }

    #[test]
    fn search_ends_by_accepting_or_cancelling() {
        let h = history();
        let mut s = Search::new("draft".into());
        assert_eq!(s.key(key(KeyCode::Enter), &h), Step::Accept);
        assert_eq!(s.key(key(KeyCode::Esc), &h), Step::Cancel);
        assert_eq!(s.key(ctrl('g'), &h), Step::Cancel);
        assert_eq!(s.key(key(KeyCode::Left), &h), Step::AcceptAndPass);
        assert_eq!(s.saved, "draft");
    }

    #[test]
    fn the_status_grows_as_the_session_does() {
        let mut meter = Meter {
            context: 1_050_000,
            estimate: true,
            ..Meter::default()
        };
        assert_eq!(meter.status("gpt-5.6-luna"), "gpt-5.6-luna");
        meter.add(Usage {
            cost: Some(0.005),
            ..Usage::from_parts(11_000, 500)
        });
        assert_eq!(
            meter.status("gpt-5.6-luna"),
            "gpt-5.6-luna  11.5k/1.05M  ~$0.0050"
        );
    }

    /// Context used carries over to the next prompt; the per-prompt counts do not.
    #[test]
    fn finishing_a_prompt_resets_only_its_own_counts() {
        let mut meter = Meter {
            context: 1_000,
            ..Meter::default()
        };
        assert_eq!(meter.finish(), None);
        meter.add(Usage::from_parts(10, 2));
        assert_eq!(
            meter.finish().as_deref(),
            Some("12/1k context, 10 in, 2 out")
        );
        assert_eq!(meter.finish(), None);
        assert_eq!(meter.used, 12);
    }

    #[test]
    fn counts_are_short() {
        assert_eq!(count(508), "508");
        assert_eq!(count(39_317), "39.3k");
        assert_eq!(count(400_000), "400k");
        assert_eq!(count(1_050_000), "1.05M");
        assert_eq!(count(1_000_000), "1M");
    }

    #[test]
    fn the_usage_line_adds_cost_only_when_known() {
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
