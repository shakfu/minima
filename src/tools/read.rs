//! No path jail. The sandbox bounds writes, not reads, and `bash` reads the whole filesystem,
//! so a jail here would only push the model through `cat`.

use anyhow::{Context, Result, anyhow, bail};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

use crate::config::TOOL_OUTPUT_CAP;

const MAX_LINES: usize = 2000;
/// Bytes kept from one line. A line past the cap on the whole result is already more than the
/// model will see.
const LINE_CAP: usize = TOOL_OUTPUT_CAP;

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

/// Streams the file, and keeps at most the cap on a tool result, so memory follows neither the
/// size of the file nor the length of a line in it.
pub async fn call(args: Args) -> Result<String> {
    let file = tokio::fs::File::open(&args.path)
        .await
        .with_context(|| format!("reading {}", args.path))?;
    // A character device has no end, so the scan below would not finish. Directories reach here
    // too, and `EISDIR` from the first read says less than this does.
    let meta = file
        .metadata()
        .await
        .with_context(|| format!("reading {}", args.path))?;
    if !meta.is_file() {
        bail!("{} is not a regular file", args.path);
    }

    let start = args.offset.unwrap_or(1).max(1);
    let limit = args.limit.unwrap_or(MAX_LINES).min(MAX_LINES);
    let end = start.saturating_add(limit);

    let mut reader = BufReader::new(file);
    let (mut out, mut shown, mut total) = (String::new(), 0, 0);
    let mut cut = false;
    while let Some((line, line_cut)) = next_line(&mut reader, &args.path).await? {
        total += 1;
        // Lines are still counted past the cap, so the footer says how much was left. Only the
        // text stops accumulating.
        if (start..end).contains(&total) && out.len() < TOOL_OUTPUT_CAP {
            out.push_str(&format!("{total:>6}\t{line}\n"));
            cut |= line_cut;
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
    if cut {
        out.push_str(&format!("... a line was cut at {LINE_CAP} bytes\n"));
    }
    Ok(out)
}

/// One line without its `\n` or `\r\n`, and whether it was cut at `LINE_CAP`. `Ok(None)` at EOF.
///
/// Written out rather than `lines()`, which grows one allocation until it finds a newline: a file
/// that has none, such as a minified bundle, is read into memory whole.
async fn next_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    path: &str,
) -> Result<Option<(String, bool)>> {
    let mut line: Vec<u8> = Vec::new();
    let (mut any, mut cut) = (false, false);
    loop {
        let buf = reader
            .fill_buf()
            .await
            .map_err(|e| anyhow!(e).context(format!("reading {path}")))?;
        if buf.is_empty() {
            break;
        }
        any = true;
        let (take, consumed, eol) = match buf.iter().position(|&b| b == b'\n') {
            Some(i) => (i, i + 1, true),
            None => (buf.len(), buf.len(), false),
        };
        // Past the cap the bytes are consumed and dropped, so the scan still reaches the newline.
        let room = LINE_CAP.saturating_sub(line.len());
        cut |= take > room;
        line.extend_from_slice(&buf[..take.min(room)]);
        reader.consume(consumed);
        if eol {
            break;
        }
    }
    if !any {
        return Ok(None);
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    let text = match String::from_utf8(line) {
        Ok(text) => text,
        // A cut can land inside a character, so a cut line is repaired rather than refused.
        Err(e) if cut => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        Err(_) => bail!("{path} is not UTF-8 text"),
    };
    Ok(Some((text, cut)))
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

    /// `lines()` would hold the whole line before returning it, so a file with no newline in it
    /// is a way to exhaust memory.
    #[tokio::test]
    async fn one_enormous_line_is_cut_rather_than_held() {
        let dir = Scratch::new("read-long-line");
        let path = dir.file("min.js");
        std::fs::write(&path, "x".repeat(LINE_CAP * 3)).unwrap();

        let out = read(&path, None, None).await.unwrap();
        assert!(out.len() < LINE_CAP + 128, "kept {} bytes", out.len());
        assert!(out.contains("a line was cut"), "the cut was not reported");
    }

    /// `/dev/zero` never reaches a newline or an end.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_file_that_is_not_a_file_is_refused() {
        let err = read("/dev/zero", None, None).await.unwrap_err();
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[tokio::test]
    async fn output_stops_at_the_cap_and_says_what_is_left() {
        let dir = Scratch::new("read-cap");
        let path = dir.file("big.txt");
        let line = "y".repeat(200);
        std::fs::write(&path, format!("{line}\n").repeat(500)).unwrap();

        let out = read(&path, None, None).await.unwrap();
        assert!(out.len() < TOOL_OUTPUT_CAP * 2, "kept {} bytes", out.len());
        assert!(out.contains("more lines"), "{out:?}");
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
