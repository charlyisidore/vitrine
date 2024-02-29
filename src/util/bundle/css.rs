//! Bundle CSS code.
//!
//! This module uses [`lightningcss`] under the hood.

use std::path::Path;

use lightningcss::bundler::{Bundler, FileProvider};
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
}

/// Bundle a CSS file.
pub fn bundle_css(path: impl AsRef<Path>) -> Result<String, BundleCssError> {
    let path = path.as_ref();

    let file_provider = FileProvider::new();

    let parser_options = Default::default();

    let mut bundler = Bundler::new(&file_provider, None, parser_options);

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
    use temp_dir::TempDir;

    use super::bundle_css;

    #[test]
    fn bundle() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();
        let path = dir.join("foo.css");

        std::fs::write(&path, r#"@import url("bar.css");"#).expect("failed to create file");

        std::fs::write(
            dir.join("bar.css"),
            concat!(
                ".foo {\n",          //
                "  color: black;\n", //
                "}\n"
            ),
        )
        .expect("failed to create file");

        let result = bundle_css(path).unwrap();

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
