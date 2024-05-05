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

/// Convert a URL to an output file path w.r.t. given base directory.
pub fn url_to_path(url: impl AsRef<str>, base_dir: impl AsRef<Path>) -> PathBuf {
    let url = UrlPath::from(url.as_ref());
    let base_dir = base_dir.as_ref().to_path_buf();

    url.segments()
        .filter(|s| !s.is_empty())
        .fold(base_dir, |path, segment| path.join(segment))
}

/// Pipeline task.
pub mod task {
    use std::path::{Path, PathBuf};

    use super::{normalize_page_url, url_to_path, OutputError};
    use crate::{
        build::{Asset, Page, Xml},
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
                "path must be inside `config.output_dir`"
            );

            println!("Writing {:?}", path);

            std::fs::create_dir_all(path.parent().unwrap())?;
            Ok(std::fs::write(path, content)?)
        }
    }

    impl Task<(Page, Asset, Xml), (PathBuf,), OutputError> for OutputTask<'_> {
        fn process(
            self,
            (rx_page, rx_asset, rx_xml): (Receiver<Page>, Receiver<Asset>, Receiver<Xml>),
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

            for asset in rx_asset {
                let (url, content) = match asset {
                    Asset::Image(image) => (image.url, None),
                    Asset::Script(script) => (script.url, Some(script.content)),
                    Asset::Style(style) => (style.url, Some(style.content)),
                };
                let output_path = url_to_path(url, output_dir);
                if let Some(content) = content {
                    self.write(&output_path, content)?;
                }
                tx.send(output_path).unwrap();
            }

            for xml in rx_xml {
                let output_path = url_to_path(&xml.url, output_dir);
                self.write(&output_path, xml.content)?;
                tx.send(output_path).unwrap();
            }

            Ok(())
        }
    }
}
