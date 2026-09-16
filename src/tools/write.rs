use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Path to write, absolute or relative to the working directory.
    pub path: String,
    /// Full contents of the file. Any existing file is replaced.
    pub content: String,
}

pub async fn call(args: Args) -> Result<String> {
    if let Some(parent) = std::path::Path::new(&args.path).parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let bytes = args.content.len();
    tokio::fs::write(&args.path, &args.content)
        .await
        .with_context(|| format!("writing {}", args.path))?;
    Ok(format!("wrote {bytes} bytes to {}", args.path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    #[tokio::test]
    async fn creates_missing_parents_and_replaces_existing_content() {
        let dir = Scratch::new("write");
        let path = dir.file("a/b/f.txt");

        for content in ["first", "second"] {
            call(Args {
                path: path.clone(),
                content: content.into(),
            })
            .await
            .unwrap();
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }
}
