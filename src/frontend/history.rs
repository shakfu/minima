//! Prompt history in `history.txt`, one entry per line, newest last.
//!
//! The file keeps reedline's format, which minima used before, so existing history carries over:
//! a newline inside an entry is written as `<\n>`.

use std::io::Write;
use std::path::PathBuf;

const CAPACITY: usize = 1000;
const NEWLINE: &str = "<\\n>";

pub struct History {
    path: Option<PathBuf>,
    entries: Vec<String>,
    /// The entry shown while browsing; `entries.len()` when not browsing.
    at: usize,
    /// What was typed before browsing began, given back when browsing runs off the newest end.
    draft: String,
}

impl History {
    /// Entries in `skip` are dropped, so exit commands from older sessions do not come back.
    /// The file is rewritten only when that or the capacity removed something.
    pub fn load(path: Option<PathBuf>, skip: &[&str]) -> Self {
        let raw = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let lines: Vec<&str> = raw.lines().collect();
        let mut entries: Vec<String> = lines
            .iter()
            .filter(|line| !line.is_empty() && !skip.contains(line))
            .map(|line| line.replace(NEWLINE, "\n"))
            .collect();
        entries.drain(..entries.len().saturating_sub(CAPACITY));
        let history = Self {
            at: entries.len(),
            path,
            entries,
            draft: String::new(),
        };
        if history.entries.len() < lines.len() {
            history.rewrite();
        }
        history
    }

    /// Records a submitted prompt, unless it is empty or repeats the one before it.
    pub fn push(&mut self, entry: &str) {
        let repeat = self.entries.last().is_some_and(|last| last == entry);
        if !entry.trim().is_empty() && !repeat {
            self.entries.push(entry.to_string());
            self.append(entry);
            if self.entries.len() > CAPACITY {
                self.entries.remove(0);
            }
        }
        self.at = self.entries.len();
    }

    /// One entry older. `current` is kept as the draft when browsing starts.
    pub fn prev(&mut self, current: &str) -> Option<String> {
        if self.at == 0 {
            return None;
        }
        if self.at == self.entries.len() {
            self.draft = current.to_string();
        }
        self.at -= 1;
        Some(self.entries[self.at].clone())
    }

    /// One entry newer, then the draft.
    pub fn next(&mut self) -> Option<String> {
        if self.at >= self.entries.len() {
            return None;
        }
        self.at += 1;
        Some(
            self.entries
                .get(self.at)
                .cloned()
                .unwrap_or_else(|| std::mem::take(&mut self.draft)),
        )
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, index: usize) -> &str {
        &self.entries[index]
    }

    /// The newest entry before `before` that contains `query`.
    pub fn search(&self, query: &str, before: usize) -> Option<usize> {
        self.entries[..before.min(self.entries.len())]
            .iter()
            .rposition(|entry| entry.contains(query))
    }

    /// Prompts are written verbatim, so the file is owner-only. Best effort: history that cannot
    /// be written is not a reason to fail the run.
    fn append(&self, entry: &str) {
        let Some(path) = &self.path else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = crate::config::restrict_to_owner(parent);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path);
        if let Ok(mut file) = file {
            let _ = crate::config::restrict_to_owner(path);
            let _ = writeln!(file, "{}", entry.replace('\n', NEWLINE));
        }
    }

    /// Replaced by rename, and tightened before it, as the model cache is.
    fn rewrite(&self) {
        let Some(path) = &self.path else { return };
        let body: String = self
            .entries
            .iter()
            .map(|e| format!("{}\n", e.replace('\n', NEWLINE)))
            .collect();
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, body).is_ok() {
            let _ = crate::config::restrict_to_owner(&tmp);
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> (crate::tools::Scratch, PathBuf) {
        let dir = crate::tools::Scratch::new(name);
        let path = PathBuf::from(dir.file("history.txt"));
        (dir, path)
    }

    #[test]
    fn entries_survive_a_restart_with_their_newlines() {
        let (_dir, path) = scratch("history-roundtrip");
        let mut h = History::load(Some(path.clone()), &[]);
        h.push("one");
        h.push("two\nlines");
        h.push("two\nlines");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "one\ntwo<\\n>lines\n"
        );
        let mut h = History::load(Some(path), &[]);
        assert_eq!(h.prev("").as_deref(), Some("two\nlines"));
        assert_eq!(h.prev("").as_deref(), Some("one"));
    }

    #[test]
    fn skipped_entries_are_dropped_from_the_file_and_paths_kept() {
        let (_dir, path) = scratch("history-skip");
        std::fs::write(&path, "fix it\n/quit\n/etc/hosts is wrong\n/exit\n").unwrap();
        let mut h = History::load(Some(path.clone()), &["/quit", "/exit"]);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "fix it\n/etc/hosts is wrong\n"
        );
        assert_eq!(h.prev("").as_deref(), Some("/etc/hosts is wrong"));
    }

    #[test]
    fn browsing_past_the_newest_entry_gives_back_the_draft() {
        let mut h = History::load(None, &[]);
        h.push("old");
        assert_eq!(h.next(), None);
        assert_eq!(h.prev("half typed").as_deref(), Some("old"));
        assert_eq!(h.prev("old"), None);
        assert_eq!(h.next().as_deref(), Some("half typed"));
        assert_eq!(h.next(), None);
    }

    #[test]
    fn only_the_newest_entries_are_kept() {
        let (_dir, path) = scratch("history-capacity");
        let body: String = (0..CAPACITY + 5).map(|i| format!("p{i}\n")).collect();
        std::fs::write(&path, body).unwrap();
        let mut h = History::load(Some(path.clone()), &[]);
        let kept = std::fs::read_to_string(&path).unwrap();
        assert_eq!(kept.lines().count(), CAPACITY);
        assert!(kept.starts_with("p5\n"));
        assert_eq!(
            h.prev("").as_deref(),
            Some(format!("p{}", CAPACITY + 4).as_str())
        );
    }
}
