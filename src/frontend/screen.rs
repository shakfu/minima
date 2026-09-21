//! The terminal while the REPL runs.
//!
//! An inline viewport at the bottom holds three things: the row in progress, the input box and
//! the status bar. Finished rows go above it into the terminal's own scrollback, so they scroll,
//! copy and outlive minima like any other output.
//!
//! Streaming text is word-wrapped and every complete row is committed at once. Greedy wrapping
//! never moves an earlier row once a later one exists, so only the last row waits.

use std::io::Stdout;
use std::ops::Range;
use std::time::Instant;

use anyhow::Result;
use crossterm::{event, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use ratatui_textarea::{CursorMove, TextArea};
use unicode_width::UnicodeWidthChar;

use crate::theme;

/// The input box grows with its text up to this many rows, then scrolls.
const MAX_INPUT_ROWS: u16 = 6;
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// How a committed row looks: the roles `theme` names, plus the banner and the echoed prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Title,
    Prompt,
    Role(theme::Style),
}

pub struct Screen {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    height: u16,
    pub input: TextArea<'static>,
    /// Assistant text past the last committed row.
    pending: String,
    /// Shown in the top row in place of `pending` while a tool runs.
    activity: Option<String>,
    /// Right side of the status bar.
    pub status: String,
    /// Set while a turn runs; the status bar then shows a spinner and the elapsed time.
    pub busy_since: Option<Instant>,
    /// Ctrl-R's prompt, shown on the left of the status bar while a search runs.
    pub search: Option<String>,
    cwd: String,
}

impl Screen {
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(std::io::stdout(), event::EnableBracketedPaste)?;
        let input = new_input("");
        let height = height_for(&input);
        Ok(Self {
            terminal: inline(height)?,
            height,
            input,
            pending: String::new(),
            activity: None,
            status: String::new(),
            busy_since: None,
            search: None,
            cwd: working_dir(),
        })
    }

    /// Replaces the input text, with the cursor at its end.
    pub fn set_input(&mut self, text: &str) {
        self.input = new_input(text);
    }

    pub fn input_text(&self) -> String {
        self.input.lines().join("\n")
    }

    pub fn set_activity(&mut self, activity: Option<String>) {
        self.activity = activity;
    }

    /// Assistant text as it streams.
    pub fn text(&mut self, delta: &str) -> Result<()> {
        self.pending.push_str(&delta.replace('\t', "    "));
        let width = self.width();
        let mut rows = Vec::new();
        while let Some(end) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=end).collect();
            let line = &line[..end];
            rows.extend(wrap(line, width).into_iter().map(|r| line[r].to_string()));
        }
        // Every row of the unfinished line but the last is final.
        let wrapped = wrap(&self.pending, width);
        if let [done @ .., last] = wrapped.as_slice() {
            rows.extend(done.iter().map(|r| self.pending[r.clone()].to_string()));
            let rest = self.pending[last.start..].to_string();
            self.pending = rest;
        }
        self.commit(rows.into_iter().map(Line::from).collect())
    }

    /// A whole line in `tone`, after any unfinished assistant text.
    pub fn line(&mut self, tone: Tone, text: &str) -> Result<()> {
        self.spans(vec![(tone, text.to_string())])
    }

    /// One logical line built from differently styled parts. Wrapped as a whole.
    pub fn spans(&mut self, parts: Vec<(Tone, String)>) -> Result<()> {
        self.flush()?;
        let parts: Vec<_> = parts
            .into_iter()
            .map(|(t, text)| (t, text.replace('\t', "    ")))
            .collect();
        let whole: String = parts.iter().map(|(_, t)| t.as_str()).collect();
        let mut rows = Vec::new();
        let mut base = 0;
        for line in whole.split('\n') {
            for row in wrap(line, self.width()) {
                rows.push(styled(&parts, base + row.start..base + row.end, &whole));
            }
            base += line.len() + 1;
        }
        self.commit(rows)
    }

    /// Ends the unfinished assistant line, if any.
    pub fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let row = std::mem::take(&mut self.pending);
        self.commit(vec![Line::from(row)])
    }

    pub fn draw(&mut self) -> Result<()> {
        self.fit()?;
        let top = match &self.activity {
            Some(activity) => Line::styled(activity.clone(), tone(Tone::Role(theme::Style::Muted))),
            None => Line::from(self.pending.clone()),
        };
        let left = match (&self.search, self.busy_since) {
            (Some(search), _) => search.clone(),
            (None, Some(start)) => {
                let secs = start.elapsed().as_secs();
                let spin = SPINNER[(start.elapsed().as_millis() / 120) as usize % SPINNER.len()];
                format!("{spin} {secs}s  esc to cancel")
            }
            (None, None) => self.cwd.clone(),
        };
        let right = self.status.clone();
        let muted = tone(Tone::Role(theme::Style::Muted));
        self.input.set_block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(muted),
        );
        let input = &self.input;
        self.terminal.draw(|frame| {
            let [top_area, input_area, status_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            frame.render_widget(Paragraph::new(top), top_area);
            frame.render_widget(input, input_area);
            let right_width = right.chars().count() as u16;
            let [left_area, right_area] =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(right_width)])
                    .areas(status_area);
            frame.render_widget(Paragraph::new(Span::styled(left, muted)), left_area);
            frame.render_widget(Paragraph::new(Span::styled(right, muted)), right_area);
        })?;
        Ok(())
    }

    /// Leaves the scrollback as the transcript and puts the shell prompt where the viewport was.
    pub fn close(&mut self) -> Result<()> {
        self.flush()?;
        let top = self.terminal.get_frame().area().y;
        self.terminal.clear()?;
        self.terminal.set_cursor_position(Position::new(0, top))?;
        Ok(())
    }

    fn width(&self) -> usize {
        self.terminal
            .size()
            .map_or(80, |s| usize::from(s.width.max(1)))
    }

    fn commit(&mut self, rows: Vec<Line<'static>>) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let height = u16::try_from(rows.len()).unwrap_or(u16::MAX);
        self.terminal.insert_before(height, |buf| {
            let width = buf.area.width;
            for (y, row) in rows.iter().enumerate().take(usize::from(height)) {
                buf.set_line(0, y as u16, row, width);
            }
        })?;
        Ok(())
    }

    /// ratatui 0.30 cannot resize an inline viewport, so a taller or shorter input box means a new
    /// `Terminal`, started where the old viewport began.
    fn fit(&mut self) -> Result<()> {
        let want = height_for(&self.input);
        if want == self.height {
            return Ok(());
        }
        let top = self.terminal.get_frame().area().y;
        self.terminal.clear()?;
        self.terminal.set_cursor_position(Position::new(0, top))?;
        self.terminal = inline(want)?;
        self.height = want;
        Ok(())
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        crate::term::restore();
    }
}

fn inline(height: u16) -> Result<Terminal<CrosstermBackend<Stdout>>> {
    Ok(Terminal::with_options(
        CrosstermBackend::new(std::io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?)
}

/// The top row, the input rows between two rules, and the status bar.
fn height_for(input: &TextArea) -> u16 {
    let rows = u16::try_from(input.lines().len()).unwrap_or(MAX_INPUT_ROWS);
    rows.clamp(1, MAX_INPUT_ROWS) + 4
}

fn new_input(text: &str) -> TextArea<'static> {
    let mut input = TextArea::new(text.split('\n').map(str::to_string).collect());
    input.set_cursor_line_style(Style::default());
    input.set_placeholder_text("ask anything; alt-enter for a newline, /exit to leave");
    input.set_placeholder_style(tone(Tone::Role(theme::Style::Muted)));
    input.move_cursor(CursorMove::Bottom);
    input.move_cursor(CursorMove::End);
    input
}

fn tone(tone: Tone) -> Style {
    if !theme::enabled() {
        return Style::default();
    }
    match tone {
        Tone::Plain => Style::default(),
        Tone::Title => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Tone::Prompt => Style::default().add_modifier(Modifier::BOLD),
        Tone::Role(theme::Style::Muted) => Style::default().add_modifier(Modifier::DIM),
        Tone::Role(theme::Style::Warn) => Style::default().fg(Color::Yellow),
        Tone::Role(theme::Style::Error) => Style::default().fg(Color::Red),
    }
}

/// The bytes `start..end` of `whole`, the concatenated parts, keeping each part's tone.
fn styled(
    parts: &[(Tone, String)],
    Range { start, end }: Range<usize>,
    whole: &str,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut at = 0;
    for (t, text) in parts {
        let (from, to) = (at.max(start), (at + text.len()).min(end));
        if from < to {
            spans.push(Span::styled(whole[from..to].to_string(), tone(*t)));
        }
        at += text.len();
    }
    Line::from(spans)
}

/// `~/projects/x` rather than `/home/me/projects/x`.
fn working_dir() -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return String::new();
    };
    let cwd = cwd.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && cwd.starts_with(&home) => {
            format!("~{}", &cwd[home.len()..])
        }
        _ => cwd,
    }
}

/// Greedy word wrap into byte ranges of `text`, by display width. The space a row breaks at
/// belongs to neither row; a word wider than the row is split between characters.
pub fn wrap(text: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let (mut start, mut col) = (0, 0);
    let mut space: Option<usize> = None;
    for (i, c) in text.char_indices() {
        let w = c.width().unwrap_or(0);
        if col + w > width && i > start {
            if c == ' ' {
                rows.push(start..i);
                (start, col, space) = (i + 1, 0, None);
                continue;
            }
            match space.filter(|&s| s > start) {
                Some(s) => {
                    rows.push(start..s);
                    start = s + 1;
                    col = text[start..i].chars().map(|c| c.width().unwrap_or(0)).sum();
                }
                None => {
                    rows.push(start..i);
                    (start, col) = (i, 0);
                }
            }
            space = None;
        }
        if c == ' ' {
            space = Some(i);
        }
        col += w;
    }
    rows.push(start..text.len().max(start));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str, width: usize) -> Vec<&str> {
        wrap(text, width).into_iter().map(|r| &text[r]).collect()
    }

    #[test]
    fn wraps_at_spaces_and_splits_only_overlong_words() {
        assert_eq!(
            rows("hello world and more", 11),
            ["hello world", "and more"]
        );
        assert_eq!(rows("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(rows("", 10), [""]);
        assert_eq!(rows("  indented", 20), ["  indented"]);
    }

    #[test]
    fn wide_characters_count_twice() {
        assert_eq!(
            rows("\u{4f60}\u{597d}\u{4e16}\u{754c}", 4),
            ["\u{4f60}\u{597d}", "\u{4e16}\u{754c}"]
        );
    }

    /// What lets streaming commit a row early: wrapping more text never changes a row that
    /// already has one after it.
    #[test]
    fn rows_before_the_last_do_not_change_as_text_arrives() {
        let text = "the quick brown fox jumps over the lazy dog again and again";
        for width in [5, 8, 13] {
            let full = rows(text, width);
            for cut in 1..text.len() {
                let part = rows(&text[..cut], width);
                let settled = &part[..part.len() - 1];
                assert_eq!(settled, &full[..settled.len()], "width {width}, cut {cut}");
            }
        }
    }

    #[test]
    fn styled_rows_keep_each_part_its_tone() {
        let parts = vec![
            (Tone::Plain, "read a.rs".to_string()),
            (Tone::Role(theme::Style::Error), " -> gone".to_string()),
        ];
        let whole: String = parts.iter().map(|(_, t)| t.as_str()).collect();
        let line = styled(&parts, 5..whole.len(), &whole);
        let texts: Vec<_> = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(texts, ["a.rs", " -> gone"]);
    }
}
