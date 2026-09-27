//! A file changes by rename, never in place. `write` and `edit` both go through this, because the
//! destination they are pointed at is often the user's only copy.
//!
//! Two exceptions write in place, as vim's `backupcopy=auto` does: a file with other hard links,
//! which a rename would split from them, and a file whose owner a rename would change. ACLs and
//! extended attributes are not carried over. See `docs/dev/atomic-writes.md`.

use std::fs::{File, Metadata, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tokio::io::AsyncWriteExt;

/// Writes `contents` to `path`, creating missing parents. A kill, a full disk, or a failed write
/// leaves the original untouched: the destination only ever changes by rename, which is atomic.
///
/// Safe to drop at any await, which is how a cancel stops it: nothing awaits between the decision
/// to change the destination and the change, so a drop never lands after it.
pub async fn replace(path: &str, contents: &[u8]) -> Result<()> {
    let target = resolve(path).await?;
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let current = tokio::fs::metadata(&target).await.ok();
    if current
        .as_ref()
        .is_some_and(|m| m.is_file() && m.nlink() > 1)
    {
        return in_place(&target, contents);
    }
    let Some(staged) = Staged::create(&target, current.as_ref())? else {
        return in_place(&target, contents);
    };
    staged.write(contents).await?.commit()
}

/// The temp file beside the destination, removed on drop unless it was renamed into place.
struct Staged {
    tmp: PathBuf,
    target: PathBuf,
    file: Option<File>,
    renamed: bool,
}

impl Staged {
    /// `None` when the destination's owner cannot be kept, which only an in-place write preserves.
    ///
    /// Created synchronously, so a drop cannot leave a create in flight that outlives the guard.
    fn create(target: &Path, current: Option<&Metadata>) -> Result<Option<Self>> {
        let tmp = temp_path(target);
        // Never wider than the destination, including before its mode is copied below: the
        // contents land before the rename, and a private file's must not be readable meanwhile.
        let mode = current.map_or(0o666, |m| m.mode() & 0o777);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&tmp)
            .with_context(|| format!("creating {}", tmp.display()))?;
        let staged = Self {
            tmp,
            target: target.to_path_buf(),
            file: Some(file),
            renamed: false,
        };
        let Some(meta) = current else {
            return Ok(Some(staged));
        };
        let file = staged.file.as_ref().expect("just set");
        // Before the mode: a chown clears setuid and setgid.
        if !keeps_owner(file, meta)? {
            return Ok(None);
        }
        // The umask may have narrowed the mode, and `mode` dropped the special bits.
        file.set_permissions(meta.permissions())
            .with_context(|| format!("setting the mode of {}", staged.tmp.display()))?;
        Ok(Some(staged))
    }

    async fn write(mut self, contents: &[u8]) -> Result<Self> {
        let mut file = tokio::fs::File::from_std(self.file.take().expect("written once"));
        file.write_all(contents)
            .await
            .with_context(|| format!("writing {}", self.tmp.display()))?;
        // The rename publishes whatever the filesystem holds, which need not be what was written.
        file.sync_all()
            .await
            .with_context(|| format!("writing {}", self.tmp.display()))?;
        Ok(self)
    }

    /// Synchronous, so no drop lands between the rename and the result that reports it.
    fn commit(mut self) -> Result<()> {
        std::fs::rename(&self.tmp, &self.target)
            .with_context(|| format!("replacing {}", self.target.display()))?;
        self.renamed = true;
        // The rename is a change to the directory, and is durable only once the directory is
        // synced. Best effort: some filesystems refuse to sync a directory.
        let parent = self.target.parent().filter(|p| !p.as_os_str().is_empty());
        if let Ok(dir) = File::open(parent.unwrap_or(Path::new("."))) {
            let _ = dir.sync_all();
        }
        Ok(())
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if !self.renamed {
            let _ = std::fs::remove_file(&self.tmp);
        }
    }
}

/// True when the temp file has the destination's owner and group, or could be given them.
fn keeps_owner(file: &File, target: &Metadata) -> Result<bool> {
    let ours = file.metadata()?;
    if (ours.uid(), ours.gid()) == (target.uid(), target.gid()) {
        return Ok(true);
    }
    Ok(std::os::unix::fs::fchown(file, Some(target.uid()), Some(target.gid())).is_ok())
}

/// Not atomic: a crash mid-write leaves the file cut short. Synchronous for the same reason as
/// `commit`, since the destination changes from the first byte.
fn in_place(target: &Path, contents: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(target)
        .with_context(|| format!("opening {}", target.display()))?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .with_context(|| format!("writing {}", target.display()))
}

/// The symlink itself would be replaced by the rename, so writes follow it to its target and the
/// link keeps pointing where it did. A link that does not resolve is refused: its target was never
/// checked by `confine_path`, which sees only the link.
async fn resolve(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    match tokio::fs::symlink_metadata(path).await {
        Ok(meta) if meta.is_symlink() => tokio::fs::canonicalize(path).await.map_err(|e| {
            anyhow!(
                "{} is a symlink that does not resolve ({e}); write its target by name",
                path.display()
            )
        }),
        _ => Ok(path.to_path_buf()),
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
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }

        replace(&path, b"new").await.unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
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

    fn mode(path: &str) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    fn entries(dir: &Scratch) -> Vec<std::ffi::OsString> {
        std::fs::read_dir(dir.file(""))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect()
    }

    /// The contents are written before the rename, so the temp file must be private from its
    /// creation, not from the chmod that follows the write.
    #[test]
    fn a_private_file_is_staged_private_from_creation() {
        use std::os::unix::fs::PermissionsExt;
        let dir = Scratch::new("atomic-private");
        let path = dir.file("secret");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        let staged = Staged::create(Path::new(&path), Some(&meta))
            .unwrap()
            .expect("same owner");
        assert_eq!(mode(&staged.tmp.display().to_string()), 0o600);
    }

    #[test]
    fn a_dropped_stage_leaves_no_temporary() {
        let dir = Scratch::new("atomic-drop");
        let staged = Staged::create(Path::new(&dir.file("new.txt")), None)
            .unwrap()
            .expect("no owner to keep");
        assert_eq!(entries(&dir).len(), 1);
        drop(staged);
        assert!(entries(&dir).is_empty(), "{:?}", entries(&dir));
    }

    /// Without a destination to copy from, the umask decides, as for any other new file.
    #[tokio::test]
    async fn a_new_file_takes_the_default_mode() {
        let dir = Scratch::new("atomic-new-mode");
        let (plain, written) = (dir.file("plain"), dir.file("written"));
        std::fs::write(&plain, "x").unwrap();
        replace(&written, b"x").await.unwrap();
        assert_eq!(mode(&written), mode(&plain));
    }

    /// A rename would give `h1` a new inode and leave `h2` holding the old contents.
    #[tokio::test]
    async fn a_hard_linked_file_is_written_in_place() {
        let dir = Scratch::new("atomic-hardlink");
        let (h1, h2) = (dir.file("h1"), dir.file("h2"));
        std::fs::write(&h1, "a").unwrap();
        std::fs::hard_link(&h1, &h2).unwrap();

        replace(&h1, b"b").await.unwrap();

        assert_eq!(std::fs::read_to_string(&h2).unwrap(), "b");
        assert_eq!(entries(&dir).len(), 2, "{:?}", entries(&dir));
    }

    /// `confine_path` checks the link, not where it points, so following it could leave the root.
    #[tokio::test]
    async fn a_dangling_symlink_is_refused_and_kept() {
        let dir = Scratch::new("atomic-dangling");
        let link = dir.file("link.txt");
        std::os::unix::fs::symlink(dir.file("missing/real.txt"), &link).unwrap();

        let err = replace(&link, b"x").await.unwrap_err();
        assert!(err.to_string().contains("does not resolve"), "{err}");
        assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
        assert!(!Path::new(&dir.file("missing")).exists());
    }

    #[tokio::test]
    async fn missing_parents_are_created() {
        let dir = Scratch::new("atomic-parents");
        let path = dir.file("a/b/f.txt");
        replace(&path, b"x").await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }
}
