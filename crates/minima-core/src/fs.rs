//! File permissions for what minima writes under the user's config directory.

/// Tighten a path to owner-only: 0700 for a directory, 0600 for a file. Prompts and model lists
/// are written verbatim. Elsewhere than Unix the platform's default ACLs apply.
pub fn restrict_to_owner(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path)?;
        let mode = if metadata.is_dir() { 0o700 } else { 0o600 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        std::fs::metadata(path).map(drop)
    }
}
