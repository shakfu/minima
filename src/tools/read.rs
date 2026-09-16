//! No path jail. `bash` is unrestricted, so restricting the file tools would only be theatre.

use std::io::ErrorKind;

use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

const MAX_LINES: usize = 2000;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Path to the file, absolute or relative to the working directory.
    pub path: String,
    /// First line to return, 1-based. Defaults to 1.
    #[serde(default)]
    pub offset: Option<usize>,
    /// Maximum number of lines to return. Defaults to 2000.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Streams the file, so memory follows the window returned, not the size of the file.
pub async fn call(args: Args) -> Result<String> {
    let file = tokio::fs::File::open(&args.path)
        .await
        .with_context(|| format!("reading {}", args.path))?;

    let start = args.offset.unwrap_or(1).max(1);
    let limit = args.limit.unwrap_or(MAX_LINES).min(MAX_LINES);
    let end = start.saturating_add(limit);

    let mut lines = BufReader::new(file).lines();
    let (mut out, mut shown, mut total) = (String::new(), 0, 0);
    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) if e.kind() == ErrorKind::InvalidData => {
                return Err(anyhow!("{} is not UTF-8 text", args.path));
            }
            Err(e) => return Err(anyhow!(e).context(format!("reading {}", args.path))),
        };
        total += 1;
        if (start..end).contains(&total) {
            out.push_str(&format!("{total:>6}\t{line}\n"));
            shown += 1;
        }
    }

    if shown == 0 {
        return Ok(format!(
            "{} has {total} lines; none at offset {start}",
            args.path
        ));
    }
    let last = start - 1 + shown;
    if last < total {
        out.push_str(&format!("... {} more lines\n", total - last));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    async fn read(path: &str, offset: Option<usize>, limit: Option<usize>) -> Result<String> {
        call(Args {
            path: path.into(),
            offset,
            limit,
        })
        .await
    }

    fn five_lines(dir: &Scratch) -> String {
        let path = dir.file("f.txt");
        std::fs::write(&path, "a\nb\nc\nd\ne\n").unwrap();
        path
    }

    #[tokio::test]
    async fn numbers_every_line_of_a_short_file() {
        let dir = Scratch::new("read-all");
        let out = read(&five_lines(&dir), None, None).await.unwrap();
        assert_eq!(out.lines().count(), 5);
        assert_eq!(out.lines().next(), Some("     1\ta"));
        assert!(!out.contains("more lines"));
    }

    #[tokio::test]
    async fn a_window_reports_what_follows_it() {
        let dir = Scratch::new("read-window");
        let out = read(&five_lines(&dir), Some(2), Some(2)).await.unwrap();
        assert_eq!(out, "     2\tb\n     3\tc\n... 2 more lines\n");
    }

    #[tokio::test]
    async fn an_offset_past_the_end_says_how_long_the_file_is() {
        let dir = Scratch::new("read-past");
        let out = read(&five_lines(&dir), Some(9), None).await.unwrap();
        assert!(out.contains("has 5 lines; none at offset 9"), "{out}");
    }

    #[tokio::test]
    async fn binary_is_refused() {
        let dir = Scratch::new("read-binary");
        let path = dir.file("b.bin");
        std::fs::write(&path, [0xff, 0xfe, b'\n']).unwrap();
        let err = read(&path, None, None).await.unwrap_err();
        assert!(err.to_string().contains("not UTF-8"), "{err}");
    }
}
