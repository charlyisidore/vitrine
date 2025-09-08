//! Extract HTML assets.
//!
//! Extract assets (images, scripts, styles) from HTML code.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use async_channel::{Receiver, Sender};
use lol_html::{RewriteStrSettings, element, rewrite_str};

use crate::{
    Config, File, FileContent, Image, Page, ReceiverExt, Script, Style, UriReferenceString,
    UriRelativeString,
};

/// A link extracted from HTML code.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Link {
    /// An anchor element.
    Anchor {
        /// Input file path.
        path: PathBuf,
    },
    /// An image.
    Image {
        /// Input file path.
        path: PathBuf,
        /// Optional width.
        width: Option<u32>,
        /// Optional height.
        height: Option<u32>,
    },
    /// A script.
    Script {
        /// Input file path.
        path: PathBuf,
    },
    /// A stylesheet.
    Style {
        /// Input file path.
        path: PathBuf,
    },
}

/// Extract HTML assets.
pub fn run(
    config: &Config,
    page_rx: Receiver<Page>,
    page_tx: Sender<Page>,
    file_tx: Sender<File>,
    image_tx: Sender<Image>,
    script_tx: Sender<Script>,
    style_tx: Sender<Style>,
) -> Result<()> {
    let pages = page_rx.into_iter().collect::<Vec<_>>();
    let page_urls = pages
        .iter()
        .filter_map(|page| {
            page.file
                .as_ref()
                .map(|file| (file.path().to_path_buf(), page.url.clone()))
        })
        .collect::<HashMap<_, _>>();
    let mut links = HashMap::<Link, UriRelativeString>::new();

    for page in pages {
        let Some(page_dir) = page.file.as_ref().and_then(|file| file.path().parent()) else {
            page_tx.send_blocking(page)?;
            continue;
        };

        const HEIGHT: &str = "height";
        const HREF: &str = "href";
        const SRC: &str = "src";
        const WIDTH: &str = "width";

        let content = rewrite_str(&page.content, RewriteStrSettings {
            element_content_handlers: vec![element!(
                concat!(
                    r#"a[href],"#,                      //
                    r#"img[src],"#,                     //
                    r#"link[rel="stylesheet"][href],"#, //
                    r#"script[src]"#,
                ),
                |element| {
                    match element.tag_name().as_str() {
                        "a" => {
                            let href = element.get_attribute(HREF).unwrap();
                            if let Some(path) = find_file(&href, page_dir, &config.input_dir) {
                                if let Some(url) = page_urls.get(&path) {
                                    element.set_attribute(HREF, url.as_str())?;
                                } else {
                                    let link = Link::Anchor { path };
                                    if let Some(url) = links.get(&link) {
                                        element.set_attribute(HREF, url.as_str())?;
                                    } else {
                                        let url = build_url(&link, &config.input_dir)?;
                                        let href: UriReferenceString = href.try_into()?;
                                        let mut url: UriReferenceString = url.try_into()?;
                                        url.set_fragment(href.fragment());
                                        element.set_attribute(HREF, url.as_str())?;
                                        links.insert(link, url.try_into()?);
                                    }
                                }
                            }
                        },
                        "img" => {
                            let src = element.get_attribute(SRC).unwrap();
                            let width = element.get_attribute(WIDTH).and_then(|v| v.parse().ok());
                            let height = element.get_attribute(HEIGHT).and_then(|v| v.parse().ok());
                            if let Some(path) = find_file(&src, page_dir, &config.input_dir) {
                                let link = Link::Image {
                                    path,
                                    width,
                                    height,
                                };
                                if let Some(url) = links.get(&link) {
                                    element.set_attribute(SRC, url.as_str())?;
                                } else {
                                    let url = build_url(&link, &config.input_dir)?;
                                    element.set_attribute(SRC, &url)?;
                                    links.insert(link, url.try_into()?);
                                }
                            }
                        },
                        "link" => {
                            let href = element.get_attribute(HREF).unwrap();
                            if let Some(path) = find_file(&href, page_dir, &config.input_dir) {
                                let link = Link::Style { path };
                                if let Some(url) = links.get(&link) {
                                    element.set_attribute(HREF, url.as_str())?;
                                } else {
                                    let url = build_url(&link, &config.input_dir)?;
                                    element.set_attribute(HREF, &url)?;
                                    links.insert(link, url.try_into()?);
                                }
                            }
                        },
                        "script" => {
                            let src = element.get_attribute(SRC).unwrap();
                            if let Some(path) = find_file(&src, page_dir, &config.input_dir) {
                                let link = Link::Script { path };
                                if let Some(url) = links.get(&link) {
                                    element.set_attribute(SRC, url.as_str())?;
                                } else {
                                    let url = build_url(&link, &config.input_dir)?;
                                    element.set_attribute(SRC, &url)?;
                                    links.insert(link, url.try_into()?);
                                }
                            }
                        },
                        _ => {},
                    };

                    Ok(())
                }
            )],
            ..Default::default()
        })?;

        // Links have been extracted, send the page to the next step
        page_tx.send_blocking(Page { content, ..page })?;
    }

    // Create assets for extracted links to send them to the pipeline
    for (link, url) in links {
        match link {
            Link::Anchor { path } => {
                file_tx.send_blocking(File {
                    url,
                    content: FileContent::Path(path),
                })?;
            },
            Link::Image {
                path,
                width,
                height,
            } => {
                image_tx.send_blocking(Image {
                    path,
                    width,
                    height,
                    url,
                    content: Default::default(),
                })?;
            },
            Link::Script { path } => {
                script_tx.send_blocking(Script {
                    path,
                    url,
                    content: Default::default(),
                })?;
            },
            Link::Style { path } => {
                style_tx.send_blocking(Style {
                    path,
                    url,
                    content: Default::default(),
                })?;
            },
        }
    }

    Ok(())
}

/// Check if a URL refers to a source file and return its path.
fn find_file(url: &str, page_dir: &Path, input_dir: &Path) -> Option<PathBuf> {
    UriReferenceString::try_from(url)
        .ok()
        .filter(|url| url.scheme_str().is_none())
        .filter(|url| url.authority_str().is_none())
        .as_ref()
        .map(|url| url.path_str())
        .map(|path| {
            path.split('/').filter(|s| !s.is_empty()).fold(
                if path.starts_with('/') {
                    input_dir.to_path_buf()
                } else {
                    page_dir.to_path_buf()
                },
                |mut path, segment| {
                    path.push(segment);
                    path
                },
            )
        })
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| path.is_file())
        .filter(|path| path.starts_with(input_dir))
}

/// Build a URL from a [`Link`].
fn build_url(link: &Link, input_dir: &Path) -> Result<String> {
    let url = match link {
        Link::Anchor { path } => path.strip_prefix(input_dir)?.to_string_lossy().to_string(),
        Link::Image {
            path,
            width,
            height,
        } => {
            let path = path.strip_prefix(input_dir)?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let stem = if let (Some(width), Some(height)) = (width, height) {
                format!("{}--{}x{}", stem, width, height)
            } else if let Some(width) = width {
                format!("{}--{}x_", stem, width)
            } else if let Some(height) = height {
                format!("{}--_x{}", stem, height)
            } else {
                stem.to_string()
            };
            if let Some(extension) = path.extension() {
                path.with_file_name(stem).with_extension(extension)
            } else {
                path.with_file_name(stem)
            }
            .to_string_lossy()
            .to_string()
        },
        Link::Script { path } => path
            .strip_prefix(input_dir)?
            .with_extension("js")
            .to_string_lossy()
            .to_string(),
        Link::Style { path } => path
            .strip_prefix(input_dir)?
            .with_extension("css")
            .to_string_lossy()
            .to_string(),
    };

    Ok(format!("/{}", url))
}
