//! An edit that silently hits the wrong occurrence is worse than a failed edit, so an ambiguous
//! `old` is an error unless `replace_all` says otherwise.

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::Deserialize;

use super::atomic;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Path to the file to edit.
    pub path: String,
    /// Exact text to replace. Must appear exactly once unless replace_all is true.
    pub old: String,
    /// Replacement text.
    pub new: String,
    /// Replace every occurrence instead of requiring exactly one.
    #[serde(default)]
    pub replace_all: bool,
}

pub async fn call(args: Args) -> Result<String> {
    // An empty pattern matches between every character, so replace_all would rewrite the file.
    if args.old.is_empty() {
        bail!("old is empty; name the exact text to replace");
    }
    if args.old == args.new {
        bail!("old and new are identical; nothing to do");
    }
    let text = tokio::fs::read_to_string(&args.path)
        .await
        .with_context(|| format!("reading {}", args.path))?;

    // `read` shows lines without their `\r`, so a multi-line `old` taken from it cannot match a
    // CRLF file as given.
    let crlf = args.old.contains('\n') && !text.contains(&args.old) && text.contains("\r\n");
    let (old, new) = if crlf {
        (
            args.old.replace('\n', "\r\n"),
            args.new.replace('\n', "\r\n"),
        )
    } else {
        (args.old, args.new)
    };

    let hits = text.matches(&old).count();
    let updated = match (hits, args.replace_all) {
        (0, _) => bail!("{} does not contain that text", args.path),
        (n, false) if n > 1 => {
            bail!(
                "{n} occurrences in {}; pass replace_all or extend old",
                args.path
            )
        }
        (_, true) => text.replace(&old, &new),
        (_, false) => text.replacen(&old, &new, 1),
    };

    atomic::replace(&args.path, updated.as_bytes()).await?;
    Ok(format!("replaced {hits} occurrence(s) in {}", args.path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    fn args(path: &str, old: &str, new: &str, replace_all: bool) -> Args {
        Args {
            path: path.into(),
            old: old.into(),
            new: new.into(),
            replace_all,
        }
    }

    #[tokio::test]
    async fn replaces_a_unique_match() {
        let dir = Scratch::new("edit-unique");
        let path = dir.file("f.txt");
        std::fs::write(&path, "alpha beta\n").unwrap();

        call(args(&path, "beta", "gamma", false)).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "alpha gamma\n");
    }

    #[tokio::test]
    async fn an_ambiguous_match_is_refused_and_the_file_is_untouched() {
        let dir = Scratch::new("edit-ambiguous");
        let path = dir.file("f.txt");
        std::fs::write(&path, "x x\n").unwrap();

        let err = call(args(&path, "x", "y", false)).await.unwrap_err();
        assert!(err.to_string().contains("2 occurrences"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x x\n");

        call(args(&path, "x", "y", true)).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "y y\n");
    }

    #[tokio::test]
    async fn an_empty_old_is_refused_even_with_replace_all() {
        let dir = Scratch::new("edit-empty");
        let path = dir.file("f.txt");
        std::fs::write(&path, "abc\n").unwrap();

        assert!(call(args(&path, "", "X", true)).await.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "abc\n");
    }

    #[tokio::test]
    async fn a_multi_line_match_works_on_crlf_and_keeps_the_line_endings() {
        let dir = Scratch::new("edit-crlf");
        let path = dir.file("f.txt");
        std::fs::write(&path, "one\r\ntwo\r\nthree\r\n").unwrap();

        call(args(&path, "one\ntwo", "uno\ndos", false))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "uno\r\ndos\r\nthree\r\n"
        );
    }
}
