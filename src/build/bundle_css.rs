//! Bundle CSS code.
//!
//! This module uses [`lightningcss`] under the hood.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use lightningcss::bundler::{Bundler, SourceProvider};
use thiserror::Error;

use crate::util::path::PathExt;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum BundleCssError {
    /// Bundler error.
    #[error("{0}")]
    Bundler(String),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Printer error.
    #[error("{0}")]
    Printer(String),
    /// Source provider error.
    #[error("{0}")]
    SourceProvider(String),
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
}

/// Bundle a CSS file.
pub fn bundle_css_file(path: impl AsRef<Path>) -> Result<String, BundleCssError> {
    bundle_css(path, &CssSourceProvider::new())
}

/// Bundle CSS with a custom source provider.
pub fn bundle_css(
    path: impl AsRef<Path>,
    provider: &impl SourceProvider,
) -> Result<String, BundleCssError> {
    let path = path.as_ref();

    let parser_options = Default::default();

    let mut bundler = Bundler::new(provider, None, parser_options);

    let style_sheet = bundler
        .bundle(path)
        .map_err(|source| BundleCssError::Bundler(source.to_string()))?;

    let printer_options = Default::default();

    let result = style_sheet
        .to_css(printer_options)
        .map_err(|source| BundleCssError::Printer(source.to_string()))?;

    Ok(result.code)
}

/// A [`SourceProvider`] that allows to use custom source for given paths.
///
/// This provider reads CSS sources from a [`HashMap`]. It allows to store
/// custom sources associated to given paths beforehand, for example, sources
/// that have been compiled (e.g. SCSS) in memory but not written on the disk.
#[derive(Debug, Default)]
pub struct CssSourceProvider(Mutex<HashMap<PathBuf, *mut String>>);

impl CssSourceProvider {
    /// Create a source provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a custom source for given path.
    pub fn insert(&self, file: &Path, source: String) {
        let pointer = Box::into_raw(Box::new(source));
        self.0.lock().unwrap().insert(file.to_path_buf(), pointer);
    }
}

unsafe impl Sync for CssSourceProvider {}
unsafe impl Send for CssSourceProvider {}

impl Drop for CssSourceProvider {
    fn drop(&mut self) {
        for (_, pointer) in self.0.lock().unwrap().iter() {
            // SAFETY: pointers are never removed before `CssSourceProvider` is dropped
            std::mem::drop(unsafe { Box::from_raw(*pointer) });
        }
    }
}

impl SourceProvider for CssSourceProvider {
    type Error = BundleCssError;

    fn read<'a>(&'a self, file: &Path) -> Result<&'a str, Self::Error> {
        use std::collections::hash_map::Entry;

        let mut map = self.0.lock().unwrap();

        let pointer = *match map.entry(file.to_path_buf()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let source =
                    std::fs::read_to_string(file).map_err(|source| BundleCssError::WithFile {
                        source: Box::new(source.into()),
                        path: file.to_path_buf(),
                    })?;
                let pointer = Box::into_raw(Box::new(source));
                e.insert(pointer)
            },
        };

        // SAFETY: `pointer` is dropped only when `CssSourceProvider` is, and pointers
        // are never removed from `self.0`
        Ok(unsafe { &*pointer })
    }

    fn resolve(&self, specifier: &str, originating_file: &Path) -> Result<PathBuf, Self::Error> {
        Ok(originating_file.with_file_name(specifier).normalize())
    }
}

/// Pipeline task.
pub mod task {
    use super::{bundle_css, BundleCssError, CssSourceProvider};
    use crate::{
        build::Style,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to bundle CSS code.
    #[derive(Debug, Default)]
    pub struct BundleCssTask;

    impl BundleCssTask {
        /// Create a pipeline task to bundle CSS code.
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Task<(Style,), (Style,), BundleCssError> for BundleCssTask {
        fn process(
            self,
            (rx,): (Receiver<Style>,),
            (tx,): (Sender<Style>,),
        ) -> Result<(), BundleCssError> {
            let mut styles = Vec::<Style>::new();
            let provider = CssSourceProvider::new();

            for style in rx {
                if let Some(input_path) = &style.input_path {
                    provider.insert(input_path, style.content.clone());
                }
                styles.push(style);
            }

            for style in styles {
                let content = if let Some(input_path) = &style.input_path {
                    bundle_css(input_path, &provider).map_err(|source| {
                        BundleCssError::WithFile {
                            source: Box::new(source),
                            path: input_path.clone(),
                        }
                    })?
                } else {
                    style.content
                };

                tx.send(Style { content, ..style }).unwrap();
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{bundle_css, bundle_css_file, CssSourceProvider};
    use crate::util::temp_dir::TempDir;

    #[test]
    fn bundle_file() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();
        let path = dir.join("foo.css");

        std::fs::write(&path, r#"@import url("bar.css");"#).expect("failed to create file");

        std::fs::write(dir.join("bar.css"), ".foo { color: black; }")
            .expect("failed to create file");

        let result = bundle_css_file(path).unwrap();

        assert!(result.contains(".foo"));
        assert!(result.contains("color:"));
    }

    #[test]
    fn bundle_str() {
        let provider = CssSourceProvider::new();

        provider.insert(
            &PathBuf::from("foo.css"),
            r#"@import url("bar.css");"#.into(),
        );

        provider.insert(&PathBuf::from("bar.css"), ".foo { color: black; }".into());

        let result = bundle_css(PathBuf::from("foo.css"), &provider).unwrap();

        assert!(result.contains(".foo"));
        assert!(result.contains("color:"));
    }
}
