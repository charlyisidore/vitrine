//! Parse Markdown.
//!
//! [Markdown](<https://spec.commonmark.org/>) is a plain text format for writing structured
//! documents.
//!
//! This module uses [`markdown_it`] under the hood.

use markdown_it::MarkdownIt;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MarkdownError {
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
    /// The syntax can be extended using the [`Self::add_plugin`] method.
    pub fn new() -> Self {
        let mut parser = MarkdownIt::new();
        markdown_it::plugins::cmark::add(&mut parser);
        Self { parser }
    }

    /// Add a builtin plugin by name.
    ///
    /// This method returns an error when the plugin has not been found.
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
    ///
    /// See <https://docs.rs/markdown-it/0.6.0/markdown_it/plugins/index.html> for more details.
    pub fn add_plugin<S>(&mut self, name: S) -> Result<(), MarkdownError>
    where
        S: AsRef<str>,
    {
        use markdown_it::plugins::{extra, html, sourcepos};

        let name = name.as_ref();

        match name {
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
            _ => return Err(MarkdownError::PluginNotFound(name.to_string())),
        }

        Ok(())
    }

    /// Compile a Markdown string to HTML.
    pub fn parse<S>(&self, input: S) -> String
    where
        S: AsRef<str>,
    {
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
