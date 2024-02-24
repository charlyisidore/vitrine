//! Walk directories recursively.
//!
//! This module uses [`ignore`] under the hood.

use std::path::Path;

pub use ignore::DirEntry;
use ignore::WalkBuilder;

/// A directory walker.
///
/// This walker creates a recursive directory iterator that filters hidden files
/// and paths specified in `.gitignore` files.
pub struct DirWalker {
    /// Builds a recursive directory iterator.
    builder: WalkBuilder,
}

impl DirWalker {
    /// Create a directory walker.
    pub fn new<P>(dir: P) -> Self
    where
        P: AsRef<Path>,
    {
        let mut builder = WalkBuilder::new(dir);

        builder
            .hidden(true)
            .git_ignore(true)
            .ignore(false)
            .parents(false)
            .git_global(false)
            .git_exclude(false)
            .require_git(false)
            .ignore_case_insensitive(false);

        Self { builder }
    }

    /// Register a predicate to filter directories and files during the walk.
    pub fn filter_entry<P>(&mut self, predicate: P) -> &mut Self
    where
        P: Fn(&DirEntry) -> bool + Send + Sync + 'static,
    {
        self.builder.filter_entry(predicate);
        self
    }

    /// Return an iterator that yields only (valid) files.
    pub fn walk(&self) -> impl Iterator<Item = DirEntry> {
        self.builder
            .build()
            .filter_map(|result| result.ok())
            .filter(|entry| {
                entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_file())
            })
    }

    /// Return a reference to the [`WalkBuilder`] instance.
    pub fn builder_ref(&mut self) -> &WalkBuilder {
        &self.builder
    }

    /// Return a mutable reference to the [`WalkBuilder`] instance.
    pub fn builder_mut(&mut self) -> &mut WalkBuilder {
        &mut self.builder
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::DirWalker;

    #[test]
    fn hidden() {
        let temp_dir = TempDir::new();
        let dir = &temp_dir.path();

        std::fs::write(dir.join(".foo"), "").expect("failed to create file");
        std::fs::write(dir.join("bar"), "").expect("failed to create file");

        let result: Vec<PathBuf> = DirWalker::new(dir)
            .walk()
            .map(|entry| entry.path().to_path_buf())
            .collect();

        assert_eq!(result, vec![dir.join("bar")]);
    }

    #[test]
    fn git_ignore() {
        let temp_dir = TempDir::new();
        let dir = &temp_dir.path();

        std::fs::write(dir.join(".gitignore"), "foo").expect("failed to create file");
        std::fs::write(dir.join("foo"), "").expect("failed to create file");
        std::fs::write(dir.join("bar"), "").expect("failed to create file");

        let result: Vec<PathBuf> = DirWalker::new(dir)
            .walk()
            .map(|entry| entry.path().to_path_buf())
            .collect();

        assert_eq!(result, vec![dir.join("bar")]);
    }

    /// Wraps a temporary directory path.
    ///
    /// The directory is removed when this is dropped.
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).expect("failed to remove temp dir")
        }
    }

    impl TempDir {
        /// Create a temporary directory.
        fn new() -> Self {
            let dir = std::env::temp_dir();
            for _ in 0..10 {
                let path = dir.join(format!("vitrine_{}", random_number()));
                if !path.exists() {
                    std::fs::create_dir_all(&path)
                        .expect(&format!("failed to create temp dir {:?}", path));
                    return Self(path);
                }
            }
            panic!("failed to create temp dir")
        }

        /// Return the directory path.
        fn path(&self) -> &PathBuf {
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
}
