//! Bundle HTML pages.
//!
//! Extract assets from HTML code and rewrite URLs.

use std::path::PathBuf;

use lol_html::{errors::RewritingError, RewriteStrSettings};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum BundleHtmlError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// LolHtml error.
    #[error(transparent)]
    LolHtmlRewriting(#[from] RewritingError),
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

/// An asset description.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Asset {
    /// An image.
    Image(Image),
    /// A script.
    Script(Script),
    /// A stylesheet.
    Style(Style),
}

/// An image asset.
///
/// Example: `<img src="image.jpg" width="200" height="100">`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    /// URL of the image.
    pub url: String,
    /// Optional width.
    pub width: Option<u32>,
    /// Optional height.
    pub height: Option<u32>,
}

/// A script asset.
///
/// Example: `<script src="script.js"></script>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Script {
    /// URL of the script.
    pub url: String,
}

/// A style sheet asset.
///
/// Example: `<link rel="stylesheet" href="style.css">`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Style {
    /// URL of the style sheet.
    pub url: String,
}

/// Extract and optionally rewrite asset URLs from HTML code.
pub fn extract_html_assets(
    content: impl AsRef<str>,
    mut f: impl FnMut(Asset) -> Option<Asset>,
) -> Result<String, BundleHtmlError> {
    use std::convert::Infallible;
    try_extract_html_assets(content, |asset| Ok::<_, Infallible>((f)(asset)))
}

/// Same as [`extract`], with a fallible closure.
pub fn try_extract_html_assets<E>(
    content: impl AsRef<str>,
    mut f: impl FnMut(Asset) -> Result<Option<Asset>, E>,
) -> Result<String, BundleHtmlError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let content = content.as_ref();

    const HEIGHT: &str = "height";
    const HREF: &str = "href";
    const REL: &str = "rel";
    const SRC: &str = "src";
    const WIDTH: &str = "width";

    Ok(lol_html::rewrite_str(content, RewriteStrSettings {
        element_content_handlers: vec![lol_html::element!(
            concat!(
                r#"img[src],"#,                     //
                r#"link[rel="stylesheet"][href],"#, //
                r#"script[src]"#,
            ),
            |element| {
                match element.tag_name().as_str() {
                    "img" => {
                        debug_assert!(element.has_attribute(SRC));
                        let Some(src) = element.get_attribute(SRC) else {
                            return Ok(());
                        };
                        let width = element.get_attribute(WIDTH).and_then(|v| v.parse().ok());
                        let height = element.get_attribute(HEIGHT).and_then(|v| v.parse().ok());
                        if let Some(Asset::Image(image)) = f(Asset::Image(Image {
                            url: src,
                            width,
                            height,
                        }))? {
                            element.set_attribute(SRC, &image.url)?;
                            if let Some(width) = image.width {
                                element.set_attribute(WIDTH, &format!("{width}"))?;
                            } else if width.is_some() {
                                debug_assert!(element.has_attribute(WIDTH));
                                element.remove_attribute(WIDTH);
                            }
                            if let Some(height) = image.height {
                                element.set_attribute(HEIGHT, &format!("{height}"))?;
                            } else if height.is_some() {
                                debug_assert!(element.has_attribute(HEIGHT));
                                element.remove_attribute(HEIGHT);
                            }
                        }
                    },
                    "link" => {
                        debug_assert!(element.has_attribute(HREF));
                        debug_assert_eq!(
                            element.get_attribute(REL),
                            Some("stylesheet".to_string())
                        );
                        let Some(href) = element.get_attribute(HREF) else {
                            return Ok(());
                        };
                        if let Some(Asset::Style(style)) = f(Asset::Style(Style { url: href }))? {
                            element.set_attribute(HREF, &style.url)?;
                        }
                    },
                    "script" => {
                        debug_assert!(element.has_attribute(SRC));
                        let Some(src) = element.get_attribute(SRC) else {
                            return Ok(());
                        };
                        if let Some(Asset::Script(script)) = f(Asset::Script(Script { url: src }))?
                        {
                            element.set_attribute(SRC, &script.url)?;
                        }
                    },
                    _ => {},
                };

                Ok(())
            }
        )],
        ..Default::default()
    })?)
}

/// Pipeline task.
pub mod task {
    use std::{
        collections::BTreeSet,
        path::{Path, PathBuf},
    };

    use super::{try_extract_html_assets, Asset, BundleHtmlError};
    use crate::{
        build::{Image, Page, Script, Style},
        config::Config,
        util::{
            pipeline::{Receiver, Sender, Task},
            url::UrlPath,
        },
    };

    /// Task to extract assets and bundle HTML pages.
    #[derive(Debug)]
    pub struct BundleHtmlTask<'config> {
        config: &'config Config,
    }

    impl<'config> BundleHtmlTask<'config> {
        /// Create a pipeline task to bundle HTML pages.
        pub fn new(config: &'config Config) -> BundleHtmlTask<'config> {
            Self { config }
        }

        /// Return the input path of an asset, if it exists and inside
        /// `config.input_dir`.
        fn asset_input_path(&self, page: &Page, url: &UrlPath) -> Option<PathBuf> {
            let mut path = if url.is_absolute() {
                // Input directory
                self.config.input_dir.clone()
            } else {
                // Page's directory
                page.input_path
                    .parent()
                    .expect("`input_path` must have a parent")
                    .to_owned()
            };

            for segment in url.segments().filter(|s| !s.is_empty()) {
                path.push(segment);
            }

            if !path.exists() {
                return None;
            }

            let path = path.canonicalize().expect("path must exist");

            // Ensure the asset is inside `input_dir`
            if !path.starts_with(&self.config.input_dir) {
                return None;
            }

            Some(path)
        }

        /// Return the URL of an asset.
        fn asset_url(&self, input_path: &Path, extension: Option<&str>) -> UrlPath {
            let path = input_path
                .strip_prefix(&self.config.input_dir)
                .expect("path must be descendent of `config.input_dir`");

            let path = if let Some(extension) = extension {
                path.with_extension(extension)
            } else {
                path.to_owned()
            };

            UrlPath::from(format!("/{}", path.to_str().expect("path must be unicode")))
        }
    }

    impl Task<(Page,), (Page, Image, Script, Style), BundleHtmlError> for BundleHtmlTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (page_tx, image_tx, script_tx, style_tx): (
                Sender<Page>,
                Sender<Image>,
                Sender<Script>,
                Sender<Style>,
            ),
        ) -> Result<(), BundleHtmlError> {
            let mut assets = BTreeSet::<(PathBuf, Asset)>::new();

            for page in rx {
                try_extract_html_assets(&page.content, |asset| -> Result<_, BundleHtmlError> {
                    match asset {
                        Asset::Image(mut image) => {
                            let url = image.url.clone().into();
                            let Some(input_path) = self.asset_input_path(&page, &url) else {
                                return Ok(None);
                            };
                            image.url = self.asset_url(&input_path, None).to_string();
                            assets.insert((input_path, Asset::Image(image.clone())));
                            Ok(Some(Asset::Image(image)))
                        },
                        Asset::Script(mut script) => {
                            let url = script.url.clone().into();
                            let Some(input_path) = self.asset_input_path(&page, &url) else {
                                return Ok(None);
                            };
                            script.url = self.asset_url(&input_path, Some("js")).to_string();
                            assets.insert((input_path, Asset::Script(script.clone())));
                            Ok(Some(Asset::Script(script)))
                        },
                        Asset::Style(mut style) => {
                            let url = style.url.clone().into();
                            let Some(input_path) = self.asset_input_path(&page, &url) else {
                                return Ok(None);
                            };
                            style.url = self.asset_url(&input_path, Some("css")).to_string();
                            assets.insert((input_path, Asset::Style(style.clone())));
                            Ok(Some(Asset::Style(style)))
                        },
                    }
                })
                .map_err(|source| BundleHtmlError::WithFile {
                    source: Box::new(source),
                    path: page.input_path.clone(),
                })?;

                page_tx.send(page).unwrap();
            }

            for (input_path, asset) in assets {
                match asset {
                    Asset::Image(image) => {
                        image_tx
                            .send(Image {
                                input_path,
                                url: image.url.into(),
                            })
                            .unwrap();
                    },
                    Asset::Script(script) => {
                        let content = std::fs::read_to_string(&input_path)
                            .map_err(|source| BundleHtmlError::WithFile {
                                source: Box::new(source.into()),
                                path: input_path.clone(),
                            })
                            .map_err(|source| BundleHtmlError::WithUrl {
                                source: Box::new(source),
                                url: script.url.clone(),
                            })?;
                        script_tx
                            .send(Script {
                                input_path,
                                url: script.url.into(),
                                content,
                            })
                            .unwrap();
                    },
                    Asset::Style(style) => {
                        let content = std::fs::read_to_string(&input_path)
                            .map_err(|source| BundleHtmlError::WithFile {
                                source: Box::new(source.into()),
                                path: input_path.clone(),
                            })
                            .map_err(|source| BundleHtmlError::WithUrl {
                                source: Box::new(source),
                                url: style.url.clone(),
                            })?;
                        style_tx
                            .send(Style {
                                input_path,
                                url: style.url.into(),
                                content,
                            })
                            .unwrap();
                    },
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_html_assets, Asset, Image, Script, Style};

    const INPUT: &str = concat!(
        "<html>\n",                                                   //
        "  <head>\n",                                                 //
        "    <link href=\"style.css\" rel=\"stylesheet\">\n",         //
        "    <script src=\"script.js\"></script>\n",                  //
        "  </head>\n",                                                //
        "  <body>\n",                                                 //
        "    <img src=\"image.jpg\" width=\"200\" height=\"100\">\n", //
        "  </body>\n",                                                //
        "</html>\n"
    );

    #[test]
    fn extract_assets() {
        let mut result = Vec::new();

        extract_html_assets(INPUT, |asset| {
            result.push(asset);
            None
        })
        .unwrap();

        assert_eq!(result, vec![
            (Asset::Style(Style {
                url: "style.css".to_string(),
            })),
            (Asset::Script(Script {
                url: "script.js".to_string()
            })),
            (Asset::Image(Image {
                url: "image.jpg".to_string(),
                width: Some(200),
                height: Some(100),
            })),
        ]);
    }

    #[test]
    fn rewrite_assets() {
        let result = extract_html_assets(INPUT, |asset| {
            Some(match asset {
                Asset::Image(image) => Asset::Image(Image {
                    url: format!("/images/{}", image.url),
                    ..image
                }),
                Asset::Script(script) => Asset::Script(Script {
                    url: format!("/scripts/{}", script.url),
                }),
                Asset::Style(style) => Asset::Style(Style {
                    url: format!("/styles/{}", style.url),
                }),
            })
        })
        .unwrap();

        assert!(result.contains("/styles/style.css"));
        assert!(result.contains("/scripts/script.js"));
        assert!(result.contains("/images/image.jpg"));
    }
}
