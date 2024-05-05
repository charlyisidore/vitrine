//! Bundle HTML pages.
//!
//! Rewrite URLs in HTML pages.

use std::path::{Path, PathBuf};

use lol_html::{errors::RewritingError, RewriteStrSettings};
use thiserror::Error;

use crate::util::url::Url;

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

/// A link.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Link {
    /// An anchor element.
    Anchor {
        /// URL of the element.
        url: String,
    },
    /// An image.
    Image {
        /// URL of the image.
        url: String,
        /// Optional width.
        width: Option<u32>,
        /// Optional height.
        height: Option<u32>,
    },
    /// A script.
    Script {
        /// URL of the script.
        url: String,
    },
    /// A stylesheet.
    Style {
        /// URL of the style sheet.
        url: String,
    },
}

/// Extract and optionally rewrite links from HTML code.
pub fn rewrite_links(
    content: impl AsRef<str>,
    mut f: impl FnMut(Link) -> Option<String>,
) -> Result<String, RewritingError> {
    let content = content.as_ref();

    const HEIGHT: &str = "height";
    const HREF: &str = "href";
    const REL: &str = "rel";
    const SRC: &str = "src";
    const WIDTH: &str = "width";

    lol_html::rewrite_str(content, RewriteStrSettings {
        element_content_handlers: vec![lol_html::element!(
            concat!(
                r#"a[href],"#,                      //
                r#"img[src],"#,                     //
                r#"link[rel="stylesheet"][href],"#, //
                r#"script[src]"#,
            ),
            |element| {
                match element.tag_name().as_str() {
                    "a" => {
                        debug_assert!(element.has_attribute(HREF));
                        let href = element.get_attribute(HREF).unwrap();
                        if let Some(url) = f(Link::Anchor { url: href }) {
                            element.set_attribute(HREF, &url)?;
                        }
                    },
                    "img" => {
                        debug_assert!(element.has_attribute(SRC));
                        let src = element.get_attribute(SRC).unwrap();
                        let width = element.get_attribute(WIDTH).and_then(|v| v.parse().ok());
                        let height = element.get_attribute(HEIGHT).and_then(|v| v.parse().ok());
                        if let Some(url) = f(Link::Image {
                            url: src,
                            width,
                            height,
                        }) {
                            element.set_attribute(SRC, &url)?;
                        }
                    },
                    "link" => {
                        debug_assert!(element.has_attribute(HREF));
                        debug_assert_eq!(
                            element.get_attribute(REL),
                            Some("stylesheet".to_string())
                        );
                        let href = element.get_attribute(HREF).unwrap();
                        if let Some(url) = f(Link::Style { url: href }) {
                            element.set_attribute(HREF, &url)?;
                        }
                    },
                    "script" => {
                        debug_assert!(element.has_attribute(SRC));
                        let src = element.get_attribute(SRC).unwrap();
                        if let Some(url) = f(Link::Script { url: src }) {
                            element.set_attribute(SRC, &url)?;
                        }
                    },
                    _ => {},
                };

                Ok(())
            }
        )],
        ..Default::default()
    })
}

/// Extract links from HTML code.
pub fn extract_links(
    content: impl AsRef<str>,
    mut f: impl FnMut(Link),
) -> Result<(), RewritingError> {
    rewrite_links(content, |link| {
        (f)(link);
        None
    })?;
    Ok(())
}

/// Check if a URL refers to a source file and return its path.
pub fn find_file_path_from_url(
    url: impl AsRef<str>,
    page_dir: impl AsRef<Path>,
    input_dir: impl AsRef<Path>,
) -> Option<PathBuf> {
    let url = Url::from(url.as_ref());
    let page_dir = page_dir.as_ref();
    let input_dir = input_dir.as_ref();

    // Skip link if it is not a simple path
    if url.scheme().is_some() || url.authority().is_some() {
        return None;
    }

    let url = url.path();

    // Base path
    let mut path = if url.is_absolute() {
        // Input directory
        input_dir.to_path_buf()
    } else {
        // Page directory
        page_dir.to_path_buf()
    };

    // Build path to the target file
    for segment in url.segments().filter(|s| !s.is_empty()) {
        path.push(segment);
    }

    // Ensure the target file is inside `config.input_dir`
    path.canonicalize()
        .ok()
        .filter(|path| path.starts_with(input_dir))
}

/// Pipeline task.
pub mod task {
    use std::collections::HashMap;

    use super::{find_file_path_from_url, rewrite_links, BundleHtmlError, Link};
    use crate::{
        build::{Asset, Image, Page, Script, Style},
        config::Config,
        util::{
            pipeline::{Receiver, Sender, Task},
            url::{Url, UrlPath},
        },
    };

    /// Task to bundle HTML pages.
    #[derive(Debug)]
    pub struct BundleHtmlTask<'config> {
        config: &'config Config,
    }

    impl<'config> BundleHtmlTask<'config> {
        /// Create a pipeline task to bundle HTML pages.
        pub fn new(config: &'config Config) -> BundleHtmlTask<'config> {
            Self { config }
        }

        /// Prepend `config.base_url` and normalize a URL.
        pub fn normalize_url(&self, url: &UrlPath) -> UrlPath {
            let url = format!("{}{}", self.config.base_url, url);
            UrlPath::from(url).normalize()
        }
    }

    impl Task<(Page, Image, Script, Style), (Page, Asset), BundleHtmlError> for BundleHtmlTask<'_> {
        fn process(
            self,
            (page_rx, image_rx, script_rx, style_rx): (
                Receiver<Page>,
                Receiver<Image>,
                Receiver<Script>,
                Receiver<Style>,
            ),
            (page_tx, asset_tx): (Sender<Page>, Sender<Asset>),
        ) -> Result<(), BundleHtmlError> {
            // Contains a mapping between links and final URLs
            // At this stage, URLs will be prepended with `config.base_url` and normalized
            let mut link_to_url = HashMap::<Link, UrlPath>::new();

            let pages: Vec<Page> = page_rx
                .into_iter()
                .map(|page| {
                    let path = page.input_path.to_str().unwrap().to_string();
                    let url = self.normalize_url(&page.url);
                    link_to_url.insert(Link::Anchor { url: path }, url.clone());
                    Page { url, ..page }
                })
                .collect();

            for image in image_rx {
                let path = image.input_path.to_str().unwrap().to_string();
                let url = self.normalize_url(&image.url);
                link_to_url.insert(
                    Link::Image {
                        url: path,
                        width: image.width,
                        height: image.height,
                    },
                    url.clone(),
                );
                asset_tx.send(Asset::Image(Image { url, ..image })).unwrap();
            }

            for script in script_rx {
                let path = script.input_path.to_str().unwrap().to_string();
                let url = self.normalize_url(&script.url);
                link_to_url.insert(Link::Script { url: path }, url.clone());
                asset_tx
                    .send(Asset::Script(Script { url, ..script }))
                    .unwrap();
            }

            for style in style_rx {
                if let Some(path) = &style.input_path {
                    let path = path.to_str().unwrap().to_string();
                    let url = self.normalize_url(&style.url);
                    link_to_url.insert(Link::Style { url: path }, url.clone());
                    asset_tx.send(Asset::Style(Style { url, ..style })).unwrap();
                } else {
                    let url = self.normalize_url(&style.url);
                    asset_tx.send(Asset::Style(Style { url, ..style })).unwrap();
                }
            }

            for page in pages {
                let page_dir = page.input_path.parent().unwrap();

                let content = rewrite_links(&page.content, |link| {
                    // Extract URL, e.g. from `href` or `src` attribute
                    let link_url = match &link {
                        Link::Anchor { url } => url,
                        Link::Image { url, .. } => url,
                        Link::Script { url } => url,
                        Link::Style { url } => url,
                    };

                    // Check if the URL refers to a source file and return its path
                    let path = find_file_path_from_url(link_url, page_dir, &self.config.input_dir)?;

                    // Convert the path to a string
                    let url = path.to_str().unwrap().to_string();

                    // Check if the link has a corresponding entry
                    let url = link_to_url.get(&match link {
                        Link::Anchor { .. } => Link::Anchor { url },
                        Link::Image { width, height, .. } => Link::Image { url, width, height },
                        Link::Script { .. } => Link::Script { url },
                        Link::Style { .. } => Link::Style { url },
                    })?;

                    // Replace the path in the original URL (preserve other parts)
                    let url = Url::from(link_url).with_path(url.as_str()).into_string();

                    Some(url)
                })
                .map_err(|source| BundleHtmlError::WithFile {
                    source: Box::new(source.into()),
                    path: page.input_path.clone(),
                })?;

                page_tx.send(Page { content, ..page }).unwrap();
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Link;

    const INPUT: &str = concat!(
        "<html>\n",                                                     //
        "  <head>\n",                                                   //
        "    <link href=\"style.css\" rel=\"stylesheet\">\n",           //
        "    <script src=\"script.js\"></script>\n",                    //
        "  </head>\n",                                                  //
        "  <body>\n",                                                   //
        "    <a href=\"example.com\">\n",                               //
        "      <img src=\"image.jpg\" width=\"200\" height=\"100\">\n", //
        "    </a>\n",                                                   //
        "  </body>\n",                                                  //
        "</html>\n"
    );

    #[test]
    fn extract_links() {
        let mut result = Vec::new();

        super::extract_links(INPUT, |link| {
            result.push(link);
        })
        .unwrap();

        assert_eq!(result, vec![
            Link::Style {
                url: "style.css".to_string(),
            },
            Link::Script {
                url: "script.js".to_string()
            },
            Link::Anchor {
                url: "example.com".to_string(),
            },
            Link::Image {
                url: "image.jpg".to_string(),
                width: Some(200),
                height: Some(100),
            },
        ]);
    }

    #[test]
    fn rewrite_links() {
        let result = super::rewrite_links(INPUT, |link| match link {
            Link::Anchor { url } => Some(format!("{url}?")),
            Link::Image { url, .. } => Some(format!("{url}?")),
            Link::Script { url } => Some(format!("{url}?")),
            Link::Style { url } => Some(format!("{url}?")),
        })
        .unwrap();

        assert_eq!(
            result,
            concat!(
                "<html>\n",                                                      //
                "  <head>\n",                                                    //
                "    <link href=\"style.css?\" rel=\"stylesheet\">\n",           //
                "    <script src=\"script.js?\"></script>\n",                    //
                "  </head>\n",                                                   //
                "  <body>\n",                                                    //
                "    <a href=\"example.com?\">\n",                               //
                "      <img src=\"image.jpg?\" width=\"200\" height=\"100\">\n", //
                "    </a>\n",                                                    //
                "  </body>\n",                                                   //
                "</html>\n"
            )
        );
    }
}
