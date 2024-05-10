//! Walk input directories.
//!
//! This module uses [`ignore`] under the hood.

use std::path::{Path, PathBuf};

pub use ignore::DirEntry;
use ignore::WalkBuilder;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum InputError {
    /// Ignore error.
    #[error(transparent)]
    Ignore(#[from] ignore::Error),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Unicode error in a path.
    #[error("invalid unicode in path")]
    InvalidUnicodePath,
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
}

/// A directory walker.
///
/// This walker creates a recursive directory iterator that filters hidden files
/// and paths specified in `.gitignore` files.
#[derive(Debug)]
pub struct DirWalker {
    /// Builds a recursive directory iterator.
    builder: WalkBuilder,
}

impl DirWalker {
    /// Create a directory walker.
    pub fn new(dir: impl AsRef<Path>) -> Self {
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
    pub fn filter_entry<P>(mut self, predicate: P) -> Self
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

/// Create a URL from given path.
pub fn path_to_url(path: impl AsRef<Path>) -> String {
    use std::path::Component;

    let path = path.as_ref();

    path.components()
        .fold(String::new(), |mut url, component| match component {
            Component::Normal(segment) => {
                url.push('/');
                url.push_str(&segment.to_string_lossy());
                url
            },
            _ => url,
        })
}

/// Normalize a page file path, e.g. by removing the extension.
pub fn normalize_page_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    if path.file_stem().is_some_and(|s| s == "index") {
        // Take directory
        // `dir/index.md` -> `dir`
        // `index.md` -> ``
        path.parent().unwrap().to_path_buf()
    } else {
        // Remove extension
        // `dir/page.md` -> `dir/page`
        path.with_extension("")
    }
}

/// Pipeline task.
pub mod task {
    use std::path::PathBuf;

    use super::{normalize_page_path, DirEntry, DirWalker, InputError};
    use crate::{
        build::{input::path_to_url, Page},
        config::Config,
        util::pipeline::{Sender, Task},
    };

    /// Task to walk input directories.
    #[derive(Debug)]
    pub struct InputTask<'config> {
        /// Vitrine configuration.
        config: &'config Config,

        /// Builds a recursive directory iterator.
        walker: DirWalker,
    }

    impl<'config> InputTask<'config> {
        /// Create a pipeline task to walk directories.
        pub fn new(config: &'config Config) -> Self {
            // Ignore special paths such as the output directory
            let mut ignore_paths: Vec<PathBuf> = Vec::new();

            // Ignore configuration file
            if let Some(config_path) = &config.config_path {
                ignore_paths.push(config_path.to_owned());
            }

            // Ignore output directory
            if let Some(output_dir) = &config.output_dir {
                ignore_paths.push(output_dir.to_owned());
            }

            // Ignore layout directory
            if let Some(layout_dir) = &config.layout_dir {
                ignore_paths.push(layout_dir.to_owned());
            }

            let walker = DirWalker::new(&config.input_dir).filter_entry(move |entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|file_name| !file_name.starts_with('_'))
                    && !ignore_paths.contains(&entry.path().to_path_buf())
            });

            Self { config, walker }
        }

        /// Determine if a [`DirEntry`] is a page.
        fn is_page(&self, entry: &DirEntry) -> bool {
            entry.path().extension().is_some_and(|extension| {
                ["html", "md"]
                    .map(Into::into)
                    .contains(&extension.to_os_string())
            })
        }

        /// Create a [`Page`] instance from a [`DirEntry`].
        fn create_page(&self, entry: DirEntry) -> Result<Page, InputError> {
            let err_with_file = |source| InputError::WithFile {
                source: Box::new(source),
                path: entry.path().to_path_buf(),
            };

            let input_path = entry
                .path()
                .canonicalize()
                .map_err(Into::into)
                .map_err(err_with_file)?;

            // `strip_prefix()` should not fail since `input_dir` is the canonical base path
            let relative_path = input_path
                .strip_prefix(&self.config.input_dir)
                .expect("entry path must be canonical and descendant of `input_dir`");

            let page_path = normalize_page_path(relative_path);
            let url = path_to_url(page_path);

            let content = std::fs::read_to_string(&input_path)
                .map_err(Into::into)
                .map_err(err_with_file)?;

            let date = entry.metadata()?.modified()?.into();

            Ok(Page {
                input_path,
                url: url.into(),
                content,
                date,
                data: Default::default(),
            })
        }
    }

    impl Task<(), (Page,), InputError> for InputTask<'_> {
        fn process(self, _: (), (tx,): (Sender<Page>,)) -> Result<(), InputError> {
            for entry in self.walker.walk() {
                if !self.is_page(&entry) {
                    continue;
                }
                let page = self.create_page(entry)?;
                tx.send(page).unwrap();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DirWalker;
    use crate::util::temp_dir::TempDir;

    #[test]
    fn walk_dir() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();

        std::fs::create_dir_all(dir.join("foo")).expect("failed to create dir");
        std::fs::write(dir.join("foo").join("bar"), "").expect("failed to create file");

        let result: Vec<_> = DirWalker::new(dir)
            .walk()
            .map(|entry| entry.path().to_path_buf())
            .collect();

        assert_eq!(result, vec![dir.join("foo").join("bar")]);
    }

    #[test]
    fn hidden() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();

        std::fs::write(dir.join(".foo"), "").expect("failed to create file");
        std::fs::write(dir.join("bar"), "").expect("failed to create file");

        let result: Vec<_> = DirWalker::new(dir)
            .walk()
            .map(|entry| entry.path().to_path_buf())
            .collect();

        assert_eq!(result, vec![dir.join("bar")]);
    }

    #[test]
    fn git_ignore() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();

        std::fs::write(dir.join(".gitignore"), "foo").expect("failed to create file");
        std::fs::write(dir.join("foo"), "").expect("failed to create file");
        std::fs::write(dir.join("bar"), "").expect("failed to create file");

        let result: Vec<_> = DirWalker::new(dir)
            .walk()
            .map(|entry| entry.path().to_path_buf())
            .collect();

        assert_eq!(result, vec![dir.join("bar")]);
    }
}
