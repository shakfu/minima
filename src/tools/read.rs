//! No path jail. `bash` is unrestricted, so restricting the file tools would only be theatre.

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::Deserialize;

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

pub async fn call(args: Args) -> Result<String> {
    let body = tokio::fs::read(&args.path)
        .await
        .with_context(|| format!("reading {}", args.path))?;
    let Ok(text) = String::from_utf8(body) else {
        bail!("{} is not UTF-8 text", args.path);
    };

    let start = args.offset.unwrap_or(1).max(1);
    let limit = args.limit.unwrap_or(MAX_LINES).min(MAX_LINES);
    let total = text.lines().count();

    let mut out = String::new();
    for (n, line) in text.lines().enumerate().skip(start - 1).take(limit) {
        out.push_str(&format!("{:>6}\t{line}\n", n + 1));
    }
    if out.is_empty() {
        return Ok(format!(
            "{} has {total} lines; none at offset {start}",
            args.path
        ));
    }
    let shown = start - 1 + out.lines().count();
    if shown < total {
        out.push_str(&format!("... {} more lines\n", total - shown));
    }
    Ok(out)
}
