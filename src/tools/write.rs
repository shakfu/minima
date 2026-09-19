use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use super::atomic;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Args {
    /// Path to write, absolute or relative to the working directory.
    pub path: String,
    /// Full contents of the file. Any existing file is replaced.
    pub content: String,
}

pub async fn call(args: Args) -> Result<String> {
    let bytes = args.content.len();
    atomic::replace(&args.path, args.content.as_bytes()).await?;
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
