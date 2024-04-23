//! Minify HTML code.
//!
//! This module uses [`minify_html`] and [`lol_html`] under the hood.

use std::string::FromUtf8Error;

use lol_html::errors::RewritingError;
use minify_html::{minify, Cfg};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MinifyHtmlError {
    /// Error converting a string in UTF-8.
    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),
    /// Error rewriting HTML.
    #[error(transparent)]
    LolHtmlRewriting(#[from] RewritingError),
    /// Error minifying CSS.
    #[error(transparent)]
    MinifyCss(#[from] crate::build::minify_css::MinifyCssError),
    /// Error minifying JavaScript.
    #[error(transparent)]
    MinifyJs(#[from] crate::build::minify_js::MinifyJsError),
    /// Error minifying `<script>` elements.
    #[error("failed to minify `<script>` element")]
    MinifyScriptElement(Box<Self>),
    /// Error minifying `<style>` elements.
    #[error("failed to minify `<style>` element")]
    MinifyStyleElement(Box<Self>),
    /// Error minifying `style` attributes.
    #[error("failed to minify `style` attribute")]
    MinifyStyleAttribute(Box<Self>),
}

/// HTML minifier.
pub struct HtmlMinifier {
    /// Configuration.
    cfg: Cfg,
}

impl HtmlMinifier {
    /// Create a HTML minifier.
    pub fn new() -> Self {
        Self {
            cfg: Cfg::spec_compliant(),
        }
    }

    /// Minify a HTML string, including inline CSS and JavaScript code.
    pub fn minify(&self, input: impl AsRef<str>) -> Result<String, MinifyHtmlError> {
        let output = self.minify_html_only(input)?;
        let output = minify_inline(output)?;
        Ok(output)
    }

    /// Minify a HTML string, excluding inline CSS and JavaScript code.
    pub fn minify_html_only(&self, input: impl AsRef<str>) -> Result<String, MinifyHtmlError> {
        let input = input.as_ref();
        let output = minify(input.as_bytes(), &self.cfg);
        Ok(String::from_utf8(output)?)
    }
}

impl Default for HtmlMinifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Minify inline CSS and JavaScript in a HTML code.
///
/// This function minifies code inside `<script>` and `<style>` elements, as
/// well as `style` attributes. The rest of the HTML code is not minified.
pub fn minify_inline(input: impl AsRef<str>) -> Result<String, MinifyHtmlError> {
    use crate::build::{minify_css::minify_css, minify_js::minify_js};

    let input = input.as_ref();
    let mut script_buffer = String::new();
    let mut style_buffer = String::new();

    Ok(lol_html::rewrite_str(
        input,
        lol_html::RewriteStrSettings {
            element_content_handlers: vec![
                lol_html::text!("script", |element| {
                    // Minify `<script>` elements
                    script_buffer.push_str(element.as_str());

                    if element.last_in_text_node() {
                        let content = minify_js(&script_buffer).map_err(|source| {
                            MinifyHtmlError::MinifyScriptElement(Box::new(source.into()))
                        })?;

                        element.set_str(content);
                        script_buffer.clear();
                    } else {
                        element.remove();
                    }

                    Ok(())
                }),
                lol_html::text!("style", |element| {
                    // Minify `<style>` elements
                    style_buffer.push_str(element.as_str());

                    if element.last_in_text_node() {
                        let content = minify_css(&style_buffer).map_err(|source| {
                            MinifyHtmlError::MinifyStyleElement(Box::new(source.into()))
                        })?;

                        element.set_str(content);
                        style_buffer.clear();
                    } else {
                        element.remove();
                    }

                    Ok(())
                }),
                lol_html::element!("*[style]", |element| {
                    // Minify `style` attributes
                    let content = element
                        .get_attribute("style")
                        .expect("element must have a `style` attribute");

                    const PREFIX: &str = "_{";
                    const SUFFIX: &str = "}";

                    // Wrap CSS rules in a fake selector to make a valid CSS stylesheet
                    let content = format!("{PREFIX}{content}{SUFFIX}");

                    let content = minify_css(content).map_err(|source| {
                        MinifyHtmlError::MinifyStyleAttribute(Box::new(source.into()))
                    })?;

                    debug_assert!(content.starts_with(PREFIX) && content.ends_with(SUFFIX));

                    element.set_attribute(
                        "style",
                        &content[PREFIX.len()..content.len() - SUFFIX.len()],
                    )?;

                    Ok(())
                }),
            ],
            ..lol_html::RewriteStrSettings::default()
        },
    )?)
}

/// Pipeline task.
pub mod task {
    use super::{HtmlMinifier, MinifyHtmlError};
    use crate::{
        build::Page,
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to minify HTML code.
    pub struct MinifyHtmlTask<'config> {
        config: &'config Config,
        minifier: HtmlMinifier,
    }

    impl<'config> MinifyHtmlTask<'config> {
        /// Create a pipeline task to minify HTML code.
        pub fn new(config: &'config Config) -> Self {
            let minifier = HtmlMinifier::new();

            Self { config, minifier }
        }
    }

    impl Task<(Page,), (Page,), MinifyHtmlError> for MinifyHtmlTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (tx,): (Sender<Page>,),
        ) -> Result<(), MinifyHtmlError> {
            for page in rx {
                let page = if self.config.optimize {
                    let content = self.minifier.minify(page.content)?;
                    Page { content, ..page }
                } else {
                    page
                };

                tx.send(page).unwrap();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HtmlMinifier;

    // Length: 219
    const INPUT: &str = concat!(
        "<html>\n",                                //
        "  <head>\n",                              //
        "    <script>\n",                          //
        "      const foo = \"bar\";\n",            //
        "    </script>\n",                         //
        "    <style>\n",                           //
        "      body {\n",                          //
        "        color: black;\n",                 //
        "      }\n",                               //
        "    </style>\n",                          //
        "  </head>\n",                             //
        "  <body style=\"background: white;\">\n", //
        "    <div>baz</div>\n",                    //
        "  </body>\n",                             //
        "</html>\n"
    );

    #[test]
    fn minify_html() {
        let minifier = HtmlMinifier::new();
        let result = minifier.minify(INPUT).unwrap();

        // Should minify `<script>`
        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
        assert!(!result.contains("const foo = \"bar\";"));
        // Should minify `<style>`
        assert!(result.contains("color:"));
        assert!(!result.contains(" color: black"));
        // Should minify `style="..."`
        assert!(result.contains("background:"));
        assert!(!result.contains("background: white;"));
        // Should minify HTML
        assert!(result.contains("<div>baz</div>"));
        assert!(!result.contains(" <div>baz</div>"));
        // Expected: 107
        assert!(result.len() <= 120);
    }

    #[test]
    fn minify_html_only() {
        let minifier = HtmlMinifier::new();
        let result = minifier.minify_html_only(INPUT).unwrap();

        // Should not minify `<script>`
        assert!(result.contains("const foo = \"bar\";"));
        // Should not minify `<style>`
        assert!(result.contains(" color: black;\n"));
        // Should not minify `style="..."`
        assert!(result.contains("background: white;"));
        // Should minify HTML
        assert!(result.contains("<div>baz</div>"));
        assert!(!result.contains(" <div>baz</div>"));
        // Expected: 133
        assert!(result.len() <= 150);
    }
}
