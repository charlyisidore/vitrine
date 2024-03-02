//! Extract HTML assets.
//!
//! Extract assets from HTML code.

use lol_html::{errors::RewritingError, RewriteStrSettings};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum AssetError {
    /// LolHtml error.
    #[error(transparent)]
    LolHtmlRewriting(#[from] RewritingError),
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
pub fn extract(
    content: impl AsRef<str>,
    mut f: impl FnMut(Asset) -> Option<Asset>,
) -> Result<String, AssetError> {
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
                        })) {
                            element.set_attribute(SRC, &image.url)?;
                            if let Some(width) = image.width {
                                element.set_attribute(WIDTH, &format!("{width}"))?;
                            } else {
                                element.remove_attribute(WIDTH);
                            }
                            if let Some(height) = image.height {
                                element.set_attribute(HEIGHT, &format!("{height}"))?;
                            } else {
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
                        if let Some(Asset::Style(style)) = f(Asset::Style(Style { url: href })) {
                            element.set_attribute(HREF, &style.url)?;
                        }
                    },
                    "script" => {
                        debug_assert!(element.has_attribute(SRC));
                        let Some(src) = element.get_attribute(SRC) else {
                            return Ok(());
                        };
                        if let Some(Asset::Script(script)) = f(Asset::Script(Script { url: src })) {
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

#[cfg(test)]
mod tests {
    use super::{extract, Asset, Image, Script, Style};

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

        extract(INPUT, |asset| {
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
        let result = extract(INPUT, |asset| {
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
