//! An edit that silently hits the wrong occurrence is worse than a failed edit, so an ambiguous
//! `old` is an error unless `replace_all` says otherwise.

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::Deserialize;

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
    if args.old == args.new {
        bail!("old and new are identical; nothing to do");
    }
    let text = tokio::fs::read_to_string(&args.path)
        .await
        .with_context(|| format!("reading {}", args.path))?;

    let hits = text.matches(&args.old).count();
    let updated = match (hits, args.replace_all) {
        (0, _) => bail!("{} does not contain that text", args.path),
        (n, false) if n > 1 => {
            bail!(
                "{n} occurrences in {}; pass replace_all or extend old",
                args.path
            )
        }
        (_, true) => text.replace(&args.old, &args.new),
        (_, false) => text.replacen(&args.old, &args.new, 1),
    };

    tokio::fs::write(&args.path, &updated)
        .await
        .with_context(|| format!("writing {}", args.path))?;
    Ok(format!("replaced {hits} occurrence(s) in {}", args.path))
}
