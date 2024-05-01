//! Parse Markdown.
//!
//! [Markdown](<https://spec.commonmark.org/>) is a plain text format for writing structured
//! documents.
//!
//! This module uses [`markdown_it`] under the hood.

pub mod syntax_highlight;
use markdown_it::MarkdownIt;
use thiserror::Error;

use crate::config::Config;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MarkdownError {
    /// Plugin not found error.
    #[error("plugin not found: `{0}`")]
    PluginNotFound(String),
}

/// Markdown parser.
#[derive(Debug)]
pub struct MarkdownParser {
    // Markdown-it parser.
    parser: MarkdownIt,
}

impl MarkdownParser {
    /// Create a Markdown parser.
    ///
    /// By default, it parses only the [CommonMark](<https://spec.commonmark.org/>) standard.
    /// The syntax can be extended using the [`Self::add_plugins`] method.
    pub fn new() -> Self {
        let mut parser = MarkdownIt::new();

        // Always add CommonMark
        markdown_it::plugins::cmark::add(&mut parser);

        Self { parser }
    }

    /// Add plugins from configuration.
    ///
    /// This method returns an error when a plugin has not been found.
    ///
    /// Available plugins:
    ///
    /// - `html`: Raw html syntax (block and inline), part of CommonMark
    ///   standard.
    /// - `strikethrough`: Strikethrough syntax (like `~~this~~`).
    /// - `beautify_links`: Pretty-print all urls and fit them into N
    ///   characters.
    /// - `linkify`: Find urls and emails, and turn them into links.
    /// - `tables`: GFM tables.
    /// - `typographer`: Common textual replacements for dashes, ©, ™, ….
    /// - `smartquotes`: Replaces `"` and `'` quotes with "nicer" ones like `‘`,
    ///   `’`, `“`, `”`, or with `’` for words like "isn't".
    /// - `heading_anchors`: Add id attribute (slug) to headings.
    /// - `sourcepos`: Add source mapping to resulting HTML, looks like this:
    ///   `<stuff data-sourcepos="1:1-2:3">`.
    /// - `syntax_highlight`: Highlight code syntax.
    ///
    /// See <https://docs.rs/markdown-it/0.6.0/markdown_it/plugins/index.html> for more details.
    pub fn add_plugins(&mut self, config: &Config) -> Result<(), MarkdownError> {
        use markdown_it::plugins::{extra, html, sourcepos};

        for plugin in &config.markdown.plugins {
            match plugin.as_str() {
                "cmark" => {},
                "html" => html::add(&mut self.parser),
                "strikethrough" => extra::strikethrough::add(&mut self.parser),
                "beautify_links" => extra::beautify_links::add(&mut self.parser),
                "linkify" => extra::linkify::add(&mut self.parser),
                "tables" => extra::tables::add(&mut self.parser),
                "typographer" => extra::typographer::add(&mut self.parser),
                "smartquotes" => extra::smartquotes::add(&mut self.parser),
                "heading_anchors" => {
                    extra::heading_anchors::add(&mut self.parser, |s| slug::slugify(s))
                },
                "sourcepos" => sourcepos::add(&mut self.parser),
                "syntax_highlight" => self::syntax_highlight::add(&mut self.parser, config),
                _ => return Err(MarkdownError::PluginNotFound(plugin.to_string())),
            }
        }

        Ok(())
    }

    /// Compile a Markdown string to HTML.
    pub fn parse(&self, input: impl AsRef<str>) -> String {
        let input = input.as_ref();
        let ast = self.parser.parse(input);
        ast.render()
    }

    /// Return a reference to the [`MarkdownIt`] instance.
    pub fn parser_ref(&mut self) -> &MarkdownIt {
        &self.parser
    }

    /// Return a mutable reference to the [`MarkdownIt`] instance.
    pub fn parser_mut(&mut self) -> &mut MarkdownIt {
        &mut self.parser
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline task.
pub mod task {
    use super::{MarkdownError, MarkdownParser};
    use crate::{
        build::Page,
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to parse Markdown.
    #[derive(Debug)]
    pub struct MarkdownTask {
        parser: MarkdownParser,
    }

    impl MarkdownTask {
        /// Create a pipeline task to parse Markdown content.
        pub fn new(config: &Config) -> Result<Self, MarkdownError> {
            let mut parser = MarkdownParser::new();
            parser.add_plugins(config)?;
            Ok(Self { parser })
        }
    }

    impl Task<(Page,), (Page,), MarkdownError> for MarkdownTask {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (tx,): (Sender<Page>,),
        ) -> Result<(), MarkdownError> {
            for page in rx {
                let page = if page
                    .input_path
                    .extension()
                    .is_some_and(|extension| extension == "md")
                {
                    let content = self.parser.parse(page.content);
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
    use super::MarkdownParser;

    #[test]
    fn parse_markdown() {
        let parser = MarkdownParser::new();
        let result = parser.parse("*Italic*");
        assert!(result.contains("<em>Italic</em>"));
    }
}
