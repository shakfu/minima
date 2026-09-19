//! A file changes by rename, never in place. `write` and `edit` both go through this, because the
//! destination they are pointed at is often the user's only copy.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;

/// Writes `contents` to `path`, creating missing parents. A kill, a full disk, or a failed write
/// leaves the original untouched: the destination only ever changes by rename, which is atomic.
pub async fn replace(path: &str, contents: &[u8]) -> Result<()> {
    let target = resolve(path).await;
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let tmp = temp_path(&target);
    let written = write_then_rename(&tmp, &target, contents).await;
    if written.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    written
}

async fn write_then_rename(tmp: &Path, target: &Path, contents: &[u8]) -> Result<()> {
    let mut file = tokio::fs::File::create(tmp)
        .await
        .with_context(|| format!("creating {}", tmp.display()))?;
    file.write_all(contents)
        .await
        .with_context(|| format!("writing {}", tmp.display()))?;
    // The rename publishes whatever the filesystem holds, which need not be what was written.
    file.sync_all()
        .await
        .with_context(|| format!("writing {}", tmp.display()))?;
    drop(file);

    // A new file takes its mode from the umask, so replacing an executable or a private file
    // would quietly change what it is.
    if let Ok(meta) = tokio::fs::metadata(target).await {
        tokio::fs::set_permissions(tmp, meta.permissions())
            .await
            .with_context(|| format!("setting the mode of {}", tmp.display()))?;
    }
    tokio::fs::rename(tmp, target)
        .await
        .with_context(|| format!("replacing {}", target.display()))
}

/// The symlink itself would be replaced by the rename, so writes follow it to its target and the
/// link keeps pointing where it did.
async fn resolve(path: &str) -> PathBuf {
    let path = Path::new(path);
    match tokio::fs::symlink_metadata(path).await {
        Ok(meta) if meta.is_symlink() => tokio::fs::canonicalize(path)
            .await
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// Beside the destination, because a rename is atomic only within one filesystem. Hidden and
/// tagged, so two writes cannot collide and a leftover is recognisable.
fn temp_path(target: &Path) -> PathBuf {
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    target.with_file_name(format!(
        ".{name}.minima-{:x}-{nanos:x}.tmp",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    #[tokio::test]
    async fn a_replaced_file_keeps_its_mode_and_leaves_no_temporary() {
        let dir = Scratch::new("atomic-mode");
        let path = dir.file("script.sh");
        std::fs::write(&path, "old").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }

        replace(&path, b"new").await.unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "the mode changed to {mode:o}");
        }
        let left: Vec<_> = std::fs::read_dir(Path::new(&path).parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "{left:?}");
    }

    /// A rename would replace the link with a regular file, orphaning what it pointed at.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlink_is_followed_rather_than_replaced() {
        let dir = Scratch::new("atomic-symlink");
        let (target, link) = (dir.file("real.txt"), dir.file("link.txt"));
        std::fs::write(&target, "old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        replace(&link, b"new").await.unwrap();

        assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }

    #[tokio::test]
    async fn missing_parents_are_created() {
        let dir = Scratch::new("atomic-parents");
        let path = dir.file("a/b/f.txt");
        replace(&path, b"x").await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }
}
