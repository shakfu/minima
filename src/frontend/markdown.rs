//! Markdown in REPL answers, one finished line at a time.
//!
//! Finished rows are already in the terminal's scrollback, so nothing that spans lines can be
//! restyled after the fact. Lines are minima's: fences, headings, rules, and list and quote markers,
//! which stay as typed. Inline markup within a line goes to `pulldown-cmark`, which gets the
//! CommonMark rules right where a hand parser would not: `snake_case`, `\*`, a `*` in a code span.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};

pub type Parts = Vec<(Style, String)>;

#[derive(Debug, Default)]
pub struct Markdown {
    /// The open fence's character and length; a closing fence needs at least as many.
    fence: Option<(char, usize)>,
    /// Rows of the current line were committed before it ended. They were plain, so the rest is
    /// rendered as inline text with no block prefix of its own.
    pub continued: bool,
}

impl Markdown {
    /// One finished line as styled parts. Ends the line.
    pub fn line(&mut self, line: &str) -> Parts {
        let continued = std::mem::take(&mut self.continued);
        if let Some((c, len)) = self.fence {
            if fence(line).is_some_and(|(d, n, info)| d == c && n >= len && info.is_empty()) {
                self.fence = None;
                return vec![(muted(), line.to_string())];
            }
            return vec![(code(), line.to_string())];
        }
        if continued {
            return inline(line, Style::default());
        }
        if let Some((c, len, _)) = fence(line) {
            self.fence = Some((c, len));
            return vec![(muted(), line.to_string())];
        }
        let (prefix, rest) = block_prefix(line);
        if prefix.is_empty() && is_rule(rest) {
            return vec![(muted(), line.to_string())];
        }
        if let Some(title) = heading(rest) {
            let mut parts = vec![(Style::default(), prefix.to_string())];
            parts.extend(inline(title, heading_style()));
            return parts;
        }
        let mut parts = vec![(Style::default(), prefix.to_string())];
        parts.extend(inline(rest, Style::default()));
        parts
    }

    /// The style the unfinished `text` can be committed in now, if it renders as typed. Plain
    /// prose, a list item's plain text, and a code block line do; anything with inline markup or
    /// a heading or fence to come does not, and waits for its newline.
    pub fn settled(&self, text: &str) -> Option<Style> {
        if self.fence.is_some() {
            return Some(code());
        }
        let rest = if self.continued {
            text
        } else {
            if fence(text).is_some() {
                return None;
            }
            let (_, rest) = block_prefix(text);
            if rest.starts_with('#') || is_rule(rest) {
                return None;
            }
            rest
        };
        (!rest.contains(SPECIAL)).then(Style::default)
    }
}

/// Characters that can start inline markup, or an entity pulldown-cmark would decode.
const SPECIAL: [char; 9] = ['*', '_', '`', '[', '<', '\\', '~', '&', '!'];

/// Leading indent plus list and quote markers, kept as typed, and the text after them.
fn block_prefix(line: &str) -> (&str, &str) {
    let mut at = line.len() - line.trim_start().len();
    loop {
        let rest = &line[at..];
        let marker = if rest.starts_with("> ") {
            2
        } else if rest.starts_with('>') {
            1
        } else if ["- ", "* ", "+ "].iter().any(|m| rest.starts_with(m)) {
            2
        } else {
            ordered_marker(rest)
        };
        if marker == 0 {
            break;
        }
        at += marker;
        at += line[at..].len() - line[at..].trim_start().len();
    }
    line.split_at(at)
}

/// The length of `1. ` or `12) `, or 0.
fn ordered_marker(text: &str) -> usize {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let after = &text.as_bytes()[digits..];
    match after {
        [b'.' | b')', b' ', ..] if (1..=9).contains(&digits) => digits + 2,
        _ => 0,
    }
}

/// A heading's title: one to six `#`, then a space or the end.
fn heading(text: &str) -> Option<&str> {
    let hashes = text.bytes().take_while(|&b| b == b'#').count();
    let rest = &text[hashes..];
    ((1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' ')))
        .then(|| rest.trim().trim_end_matches('#').trim_end())
}

/// `---`, `***` or `___`, spaces allowed between.
fn is_rule(text: &str) -> bool {
    let marks: String = text.chars().filter(|c| *c != ' ').collect();
    marks.len() >= 3
        && ["-", "*", "_"]
            .iter()
            .any(|m| marks.chars().all(|c| c.to_string() == *m))
}

/// An opening or closing fence: its character, length and info string.
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let text = line.trim_start();
    if line.len() - text.len() > 3 {
        return None;
    }
    let c = text.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let len = text.chars().take_while(|&d| d == c).count();
    let info = text[len..].trim();
    (len >= 3 && !(c == '`' && info.contains('`'))).then_some((c, len, info))
}

/// Inline markup of one line, on `base`. Leading whitespace is kept as typed.
fn inline(text: &str, base: Style) -> Parts {
    let body = text.trim_start();
    let mut out: Parts = Vec::new();
    push(&mut out, base, &text[..text.len() - body.len()]);

    let source = escape_block_start(body);
    let mut styles = vec![base];
    // Where each open link's text starts in `out`, and where it points.
    let mut links: Vec<(usize, String)> = Vec::new();
    for event in Parser::new_ext(&source, Options::ENABLE_STRIKETHROUGH) {
        let style = *styles.last().unwrap_or(&base);
        match event {
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => push(&mut out, style, &t),
            Event::Code(t) => push(&mut out, style.patch(code()), &t),
            Event::SoftBreak | Event::HardBreak => push(&mut out, style, " "),
            Event::Start(Tag::Strong) => styles.push(style.add_modifier(Modifier::BOLD)),
            Event::Start(Tag::Emphasis) => styles.push(style.add_modifier(Modifier::ITALIC)),
            Event::Start(Tag::Strikethrough) => {
                styles.push(style.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                styles.push(style.add_modifier(Modifier::UNDERLINED));
                links.push((out.len(), dest_url.to_string()));
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                styles.pop();
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                styles.pop();
                // A terminal cannot follow a link, so the target is shown unless it is the text.
                if let Some((start, url)) = links.pop() {
                    let shown: String = out[start..].iter().map(|(_, t)| t.as_str()).collect();
                    if shown != url {
                        push(&mut out, muted(), &format!(" ({url})"));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The line is parsed as a paragraph, so a character that would open a block there is escaped:
/// `#` as a heading, `>` as a quote, `-` or `+` as a list, `1.` as an ordered one. The escape
/// renders as the character. `<`, `~` and `=` are left: escaping them breaks `<https://..>` and
/// `~~struck~~`, and on one line they open nothing that renders differently.
fn escape_block_start(text: &str) -> String {
    let mut chars = text.chars();
    let first = chars.next();
    let second = chars.next();
    match (first, second) {
        (Some('#' | '>' | '+' | '-'), _) | (Some('*' | '_'), Some(' ') | None) => {
            format!("\\{text}")
        }
        (Some(c), _) if c.is_ascii_digit() && ordered_marker(&format!("{text} ")) > 0 => {
            let digits = text.bytes().take_while(u8::is_ascii_digit).count();
            format!("{}\\{}", &text[..digits], &text[digits..])
        }
        _ => text.to_string(),
    }
}

/// Appends to the last part when the style matches, so a line is a few spans, not one per event.
fn push(out: &mut Parts, style: Style, text: &str) {
    if text.is_empty() {
        return;
    }
    match out.last_mut() {
        Some((s, t)) if *s == style => t.push_str(text),
        _ => out.push((style, text.to_string())),
    }
}

fn code() -> Style {
    Style::default().fg(Color::Cyan)
}

fn muted() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn heading_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(lines: &[&str]) -> Vec<Parts> {
        let mut md = Markdown::default();
        lines.iter().map(|l| md.line(l)).collect()
    }

    fn text(parts: &Parts) -> String {
        parts.iter().map(|(_, t)| t.as_str()).collect()
    }

    /// The text of each part carrying `modifier`.
    fn with(parts: &Parts, modifier: Modifier) -> Vec<&str> {
        parts
            .iter()
            .filter(|(s, _)| s.add_modifier.contains(modifier))
            .map(|(_, t)| t.as_str())
            .collect()
    }

    #[test]
    fn emphasis_and_code_lose_their_markers_and_gain_a_style() {
        let [line] = &render(&["a **b** `c` _d_ ~~e~~"])[..] else {
            panic!()
        };
        assert_eq!(text(line), "a b c d e");
        assert_eq!(with(line, Modifier::BOLD), ["b"]);
        assert_eq!(with(line, Modifier::ITALIC), ["d"]);
        assert_eq!(with(line, Modifier::CROSSED_OUT), ["e"]);
        assert!(
            line.iter()
                .any(|(s, t)| t == "c" && s.fg == Some(Color::Cyan))
        );
    }

    /// The cases a hand parser gets wrong.
    #[test]
    fn what_is_not_markup_is_left_alone() {
        for line in [
            "call foo_bar_baz here",
            "2 * 3 * 4 = 24",
            "#hashtag, not a heading",
            "- - not a rule",
        ] {
            let [parts] = &render(&[line])[..] else {
                panic!()
            };
            assert_eq!(text(parts), line, "{line}");
            assert!(with(parts, Modifier::ITALIC).is_empty(), "{line}");
        }
        let [escaped] = &render(&[r"\*not\* `a*b*c`"])[..] else {
            panic!()
        };
        assert_eq!(text(escaped), "*not* a*b*c");
        assert!(with(escaped, Modifier::ITALIC).is_empty());
    }

    #[test]
    fn list_and_quote_markers_stay_and_their_text_is_rendered() {
        let lines = render(&["- **a:** b", "  12. c", "> *q*", "* item"]);
        let texts: Vec<_> = lines.iter().map(text).collect();
        assert_eq!(texts, ["- a: b", "  12. c", "> q", "* item"]);
        assert_eq!(with(&lines[0], Modifier::BOLD), ["a:"]);
        assert_eq!(with(&lines[2], Modifier::ITALIC), ["q"]);
    }

    #[test]
    fn a_heading_is_bold_without_its_hashes() {
        let [line] = &render(&["## Plan **now** ##"])[..] else {
            panic!()
        };
        assert_eq!(text(line), "Plan now");
        assert_eq!(with(line, Modifier::BOLD), ["Plan now"]);
    }

    /// Inside a fence nothing is markup, and only a matching fence closes it.
    #[test]
    fn a_fenced_block_is_code_until_its_own_fence_closes_it() {
        let lines = render(&["````rust", "let x = **y**;", "```", "````", "**b**"]);
        let texts: Vec<_> = lines.iter().map(text).collect();
        assert_eq!(texts, ["````rust", "let x = **y**;", "```", "````", "b"]);
        assert_eq!(lines[1][0].0, code());
        assert_eq!(lines[2][0].0, code(), "a shorter fence does not close it");
        assert_eq!(with(&lines[4], Modifier::BOLD), ["b"]);
    }

    #[test]
    fn a_link_shows_its_target_unless_the_text_is_the_target() {
        let [named, bare, struck] = &render(&[
            "see [docs](https://x.io/d)",
            "<https://x.io>",
            "~~gone~~ now",
        ])[..] else {
            panic!()
        };
        assert_eq!(text(named), "see docs (https://x.io/d)");
        assert_eq!(with(named, Modifier::UNDERLINED), ["docs"]);
        assert_eq!(text(bare), "https://x.io");
        assert_eq!(with(struck, Modifier::CROSSED_OUT), ["gone"]);
    }

    #[test]
    fn only_text_that_renders_as_typed_is_settled() {
        let md = Markdown::default();
        assert_eq!(
            md.settled("plain prose, still going"),
            Some(Style::default())
        );
        assert_eq!(md.settled("- a list item in prose"), Some(Style::default()));
        assert_eq!(md.settled("this has **bold"), None);
        assert_eq!(md.settled("## a heading"), None);
        assert_eq!(md.settled("```rust"), None);

        let mut open = Markdown::default();
        open.line("```");
        assert_eq!(open.settled("let **x** = 1;"), Some(code()));
    }
}
