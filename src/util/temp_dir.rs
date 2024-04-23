//! Create temporary directories.

use std::path::{Path, PathBuf};

/// Wraps a temporary directory path.
///
/// The directory is removed when this is dropped.
#[derive(Debug, Default)]
pub struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("failed to remove temp dir")
    }
}

impl TempDir {
    /// Create a temporary directory.
    pub fn new() -> Self {
        let dir = std::env::temp_dir();
        for _ in 0..10 {
            let path = dir.join(format!("vitrine_{}", random_number()));
            if path.exists() {
                continue;
            }
            std::fs::create_dir_all(&path)
                .unwrap_or_else(|_| panic!("failed to create temp dir {:?}", path));
            return Self(path);
        }
        panic!("failed to create temp dir")
    }

    /// Return the directory path.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Generate a random number.
fn random_number() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}
