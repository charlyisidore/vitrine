//! Extract HTML assets.
//!
//! Extract assets from HTML code and send them to the pipeline.

use std::path::PathBuf;

use lol_html::errors::RewritingError;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum AssetsError {
    /// LolHtml error.
    #[error(transparent)]
    LolHtmlRewriting(#[from] RewritingError),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
    /// Provides a URL to the context of an existing error.
    #[error("url `{url}`")]
    WithUrl {
        /// Source error.
        source: Box<Self>,
        /// URL.
        url: String,
    },
}

/// Pipeline task.
pub mod task {
    use std::{
        collections::{HashMap, HashSet},
        path::{Path, PathBuf},
    };

    use super::AssetsError;
    use crate::{
        build::{
            bundle_html::{extract_links, find_file_path_from_url, Link},
            Image, Page, Script, Style,
        },
        config::Config,
        util::{
            pipeline::{Receiver, Sender, Task},
            url::UrlPath,
        },
    };

    /// Task to extract assets and bundle HTML pages.
    #[derive(Debug)]
    pub struct AssetsTask<'config> {
        config: &'config Config,
    }

    impl<'config> AssetsTask<'config> {
        /// Create a pipeline task to bundle HTML pages.
        pub fn new(config: &'config Config) -> AssetsTask<'config> {
            Self { config }
        }

        /// Create a URL from a path with given file extension.
        fn create_url(&self, path: impl AsRef<str>, extension: Option<&str>) -> UrlPath {
            let path = PathBuf::from(path.as_ref());

            let path = path
                .strip_prefix(&self.config.input_dir)
                .expect("path must be inside `config.input_dir`");

            // Replace the file extension
            let path = if let Some(extension) = extension {
                path.with_extension(extension)
            } else {
                path.to_path_buf()
            };

            // Rebuild the URL to make it absolute
            path.components()
                .fold(UrlPath::from('/'), |mut url, component| {
                    use std::path::Component;
                    if let Component::Normal(segment) = component {
                        url.push(segment.to_str().expect("path must be unicode"));
                    }
                    url
                })
        }
    }

    impl Task<(Page,), (Page, Image, Script, Style), AssetsError> for AssetsTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (page_tx, image_tx, script_tx, style_tx): (
                Sender<Page>,
                Sender<Image>,
                Sender<Script>,
                Sender<Style>,
            ),
        ) -> Result<(), AssetsError> {
            // Contains all links found in pages
            let mut links = HashSet::<Link>::new();

            for page in rx {
                extract_links(&page.content, |link| {
                    // Extract URL, e.g. from `href` or `src` attribute
                    let url = match &link {
                        Link::Anchor { url } => url,
                        Link::Image { url, .. } => url,
                        Link::Script { url } => url,
                        Link::Style { url } => url,
                    };

                    // Check if the URL refers to a source file and return its path
                    let Some(path) = find_file_path_from_url(
                        url,
                        page.input_path.parent().unwrap(),
                        &self.config.input_dir,
                    ) else {
                        return;
                    };

                    let url = path.to_string_lossy().into();

                    links.insert(match link {
                        Link::Anchor { .. } => Link::Anchor { url },
                        Link::Image { width, height, .. } => Link::Image { url, width, height },
                        Link::Script { .. } => Link::Script { url },
                        Link::Style { .. } => Link::Style { url },
                    });
                })
                .map_err(|source| AssetsError::WithFile {
                    source: Box::new(source.into()),
                    path: page.input_path.clone(),
                })?;

                // Links have been extracted, send the page to the next step
                page_tx.send(page).unwrap();
            }

            // Memorize file contents to avoid re-reading
            let mut cache = HashMap::<PathBuf, String>::new();

            // Get file contents from the cache, otherwise read the file
            let mut read_content = |path: &Path, url: &str| -> Result<String, AssetsError> {
                if let Some(content) = cache.get(path) {
                    Ok(content.to_string())
                } else {
                    let content = std::fs::read_to_string(path)
                        .map_err(|source| AssetsError::WithFile {
                            source: Box::new(source.into()),
                            path: path.to_path_buf(),
                        })
                        .map_err(|source| AssetsError::WithUrl {
                            source: Box::new(source),
                            url: url.to_string(),
                        })?;
                    cache.insert(path.to_path_buf(), content.clone());
                    Ok(content)
                }
            };

            // Create assets for extracted links to send them to the pipeline
            for link in links {
                match link {
                    Link::Image { url, width, height } => {
                        let input_path = PathBuf::from(&url);
                        image_tx
                            .send(Image {
                                input_path,
                                width,
                                height,
                                url: self.create_url(url, None),
                            })
                            .unwrap();
                    },
                    Link::Script { url } => {
                        let input_path = PathBuf::from(&url);
                        script_tx
                            .send(Script {
                                content: read_content(&input_path, &url)?,
                                input_path,
                                url: self.create_url(url, Some("js")),
                            })
                            .unwrap();
                    },
                    Link::Style { url } => {
                        let input_path = PathBuf::from(&url);
                        style_tx
                            .send(Style {
                                content: read_content(&input_path, &url)?,
                                input_path: Some(input_path),
                                url: self.create_url(url, Some("css")),
                            })
                            .unwrap();
                    },
                    _ => {},
                }
            }

            Ok(())
        }
    }
}
