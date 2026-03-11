//! Startup initialization routines for deltav.
//!
//! Handles /data directory structure creation and configuration loading
//! from the /config volume mount.

use anyhow::{Context, Result};
use std::path::Path;

/// Required subdirectories under the data root.
const DATA_SUBDIRS: &[&str] = &["reports", "cache"];

/// Check for and create required subdirectories under `data_root`.
///
/// Uses `create_dir_all` so the call is idempotent and robust against
/// partially pre-existing directory structures.
///
/// # Arguments
///
/// * `data_root` - Root directory under which `reports/` and `cache/` are created.
///
/// # Errors
///
/// Returns an error if any subdirectory cannot be created (e.g. permission denied).
pub fn initialize_data_dir(data_root: &Path) -> Result<()> {
    for subdir in DATA_SUBDIRS {
        let path = data_root.join(subdir);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create data directory {}", path.display()))?;
        eprintln!("deltav: created {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_data_dir_creates_subdirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        initialize_data_dir(tmp.path()).unwrap();

        assert!(tmp.path().join("reports").is_dir());
        assert!(tmp.path().join("cache").is_dir());
    }

    #[test]
    fn test_initialize_data_dir_is_idempotent() {
        let tmp = tempfile::TempDir::new().unwrap();
        initialize_data_dir(tmp.path()).unwrap();
        // Second call should not error
        initialize_data_dir(tmp.path()).unwrap();

        assert!(tmp.path().join("reports").is_dir());
        assert!(tmp.path().join("cache").is_dir());
    }
}
