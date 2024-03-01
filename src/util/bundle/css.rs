//! Bundle CSS code.
//!
//! This module uses [`lightningcss`] under the hood.

use std::path::{Path, PathBuf};

use lightningcss::bundler::{Bundler, FileProvider, SourceProvider};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum BundleCssError {
    /// Bundler error.
    #[error("{0}")]
    Bundler(String),
    /// Printer error.
    #[error("{0}")]
    Printer(String),
    /// Source provider error.
    #[error("{0}")]
    SourceProvider(String),
}

/// A [`SourceProvider`] that allows to use a closure to read a source.
pub struct BundleCssProvider<'a> {
    reader: Reader<'a>,
}

/// Reader type for [`BundleCssProvider`].
pub type Reader<'a> = Box<dyn Fn(&Path) -> Result<&'a str, BundleCssError> + Send + Sync + 'a>;

impl<'a> BundleCssProvider<'a> {
    /// Create a source provider from a closure.
    pub fn new<F>(reader: F) -> Self
    where
        F: Fn(&Path) -> Result<&'a str, BundleCssError> + Send + Sync + 'a,
    {
        Self {
            reader: Box::new(reader),
        }
    }
}

impl SourceProvider for BundleCssProvider<'_> {
    type Error = BundleCssError;

    fn read<'a>(&'a self, file: &Path) -> Result<&'a str, Self::Error> {
        (self.reader)(file)
    }

    fn resolve(&self, specifier: &str, originating_file: &Path) -> Result<PathBuf, Self::Error> {
        Ok(originating_file.with_file_name(specifier))
    }
}

/// Bundle a CSS file.
pub fn bundle_css_file(path: impl AsRef<Path>) -> Result<String, BundleCssError> {
    bundle_css(path, &FileProvider::new())
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use temp_dir::TempDir;

    use super::{bundle_css, bundle_css_file, BundleCssError, BundleCssProvider};

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
        let sources: HashMap<PathBuf, String> = [
            ("foo.css".into(), r#"@import url("bar.css");"#.into()),
            ("bar.css".into(), ".foo { color: black; }".into()),
        ]
        .into();

        let provider = BundleCssProvider::new(|path| {
            sources
                .get(path)
                .map(|s| s.as_str())
                .ok_or_else(|| BundleCssError::SourceProvider("source not found".to_string()))
        });

        let result = bundle_css(PathBuf::from("foo.css"), &provider).unwrap();

        assert!(result.contains(".foo"));
        assert!(result.contains("color:"));
    }

    mod temp_dir {
        use std::path::{Path, PathBuf};

        /// Wraps a temporary directory path.
        ///
        /// The directory is removed when this is dropped.
        pub(super) struct TempDir(PathBuf);

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
                    if !path.exists() {
                        std::fs::create_dir_all(&path)
                            .expect(&format!("failed to create temp dir {:?}", path));
                        return Self(path);
                    }
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
    }
}
