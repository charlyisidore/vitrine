//! Output files.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::util::url::UrlPath;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum OutputError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convert a URL to a file path w.r.t. given base directory.
pub fn url_to_path(url: impl AsRef<str>, base_dir: impl AsRef<Path>) -> PathBuf {
    let url = UrlPath::from(url.as_ref());
    let base_dir = base_dir.as_ref().to_path_buf();

    url.segments()
        .filter(|s| !s.is_empty())
        .fold(base_dir, |path, segment| path.join(segment))
}

/// Normalize a page URL, e.g. by appending `index.html`.
pub fn normalize_page_url(mut url: String) -> String {
    if url.ends_with('/') {
        url.push_str("index.html");
        url
    } else if !url.ends_with(".html") {
        url.push_str("/index.html");
        url
    } else {
        url
    }
}

/// Pipeline task.
pub mod task {
    use std::path::{Path, PathBuf};

    use super::{normalize_page_url, url_to_path, OutputError};
    use crate::{
        build::{Image, Page, Script, Style},
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to output files.
    #[derive(Debug)]
    pub struct OutputTask<'config> {
        config: &'config Config,
    }

    impl<'config> OutputTask<'config> {
        /// Create a pipeline task to output files.
        pub fn new(config: &'config Config) -> OutputTask<'config> {
            Self { config }
        }

        /// Write a file.
        fn write(
            &self,
            path: impl AsRef<Path>,
            content: impl AsRef<str>,
        ) -> Result<(), OutputError> {
            let path = path.as_ref();
            let content = content.as_ref();

            assert!(
                path.starts_with(
                    self.config
                        .output_dir
                        .as_ref()
                        .expect("`config.output_dir` must be set")
                ),
                "path must be canonical"
            );

            println!("Writing {:?}", path);
            std::fs::create_dir_all(path.parent().expect("path must have a parent"))?;
            Ok(std::fs::write(path, content)?)
        }
    }

    impl Task<(Page, Image, Script, Style), (PathBuf,), OutputError> for OutputTask<'_> {
        fn process(
            self,
            (rx_page, rx_image, rx_script, rx_style): (
                Receiver<Page>,
                Receiver<Image>,
                Receiver<Script>,
                Receiver<Style>,
            ),
            (tx,): (Sender<PathBuf>,),
        ) -> Result<(), OutputError> {
            // Skip if no `config.output_dir` specified
            let Some(output_dir) = &self.config.output_dir else {
                return Ok(());
            };

            for page in rx_page {
                let url = normalize_page_url(page.url.to_string());
                let output_path = url_to_path(url, output_dir);
                self.write(&output_path, page.content)?;
                tx.send(output_path).unwrap();
            }

            for image in rx_image {
                let output_path = url_to_path(image.url, output_dir);
                // self.write(&output_path, page.content)?;
                tx.send(output_path).unwrap();
            }

            for script in rx_script {
                let output_path = url_to_path(script.url, output_dir);
                self.write(&output_path, script.content)?;
                tx.send(output_path).unwrap();
            }

            for style in rx_style {
                let output_path = url_to_path(style.url, output_dir);
                self.write(&output_path, style.content)?;
                tx.send(output_path).unwrap();
            }

            Ok(())
        }
    }
}
