//! Parse Markdown content.
//!
//! This module uses [`markdown_it`] under the hood.

use markdown_it::MarkdownIt;

use super::{Entry, Error};

/// Markdown parser.
pub(super) struct Parser {
    // Markdown-it parser.
    parser: MarkdownIt,
}

impl Parser {
    /// Create and configure a Markdown parser.
    pub(super) fn new() -> Self {
        let mut parser = MarkdownIt::new();
        markdown_it::plugins::cmark::add(&mut parser);
        markdown_it::plugins::html::add(&mut parser);
        markdown_it::plugins::extra::strikethrough::add(&mut parser);
        markdown_it::plugins::extra::beautify_links::add(&mut parser);
        markdown_it::plugins::extra::linkify::add(&mut parser);
        markdown_it::plugins::extra::tables::add(&mut parser);
        markdown_it::plugins::extra::typographer::add(&mut parser);
        markdown_it::plugins::extra::smartquotes::add(&mut parser);
        markdown_it::plugins::extra::heading_anchors::add(&mut parser, |s| slug::slugify(s));
        Self { parser }
    }

    /// Parse Markdown content in a [`Entry`].
    ///
    /// This function compiles the Markdown code to HTML in the `content`
    /// property. The `format` property is set to `html`.
    pub(super) fn parse_entry(&self, entry: Entry) -> Result<Entry, Error> {
        if let Some(content) = entry.content {
            let content = self.parse(content);

            return Ok(Entry {
                content: Some(content),
                format: "html".to_owned(),
                ..entry
            });
        }

        Ok(entry)
    }

    /// Parse a Markdown string and return a HTML string.
    fn parse<S>(&self, input: S) -> String
    where
        S: AsRef<str>,
    {
        let input = input.as_ref();
        let ast = self.parser.parse(&input);
        ast.render()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_common_mark() {
        const CASES: [(&str, &str); 22] = [
            ("*Italic*", "<p><em>Italic</em></p>"),
            ("_Italic_", "<p><em>Italic</em></p>"),
            ("**Bold**", "<p><strong>Bold</strong></p>"),
            ("__Bold__", "<p><strong>Bold</strong></p>"),
            ("# Heading 1", "<h1 id=\"heading-1\">Heading 1</h1>"),
            (
                concat!(
                    "Heading 1\n", //
                    "========="
                ),
                "<h1 id=\"heading-1\">Heading 1</h1>",
            ),
            ("## Heading 2", "<h2 id=\"heading-2\">Heading 2</h2>"),
            (
                concat!(
                    "Heading 2\n", //
                    "---------"
                ),
                "<h2 id=\"heading-2\">Heading 2</h2>",
            ),
            (
                "[Link](http://a.com)",
                "<p><a href=\"http://a.com\">Link</a></p>",
            ),
            (
                concat!(
                    "[Link][1]\n", //
                    "\n",          //
                    "[1]: http://b.org"
                ),
                "<p><a href=\"http://b.org\">Link</a></p>",
            ),
            (
                "![Image](http://url/a.png)",
                "<p><img src=\"http://url/a.png\" alt=\"Image\"></p>",
            ),
            (
                concat!(
                    "![Image][1]\n", //
                    "\n",            //
                    "[1]: http://url/b.jpg"
                ),
                "<p><img src=\"http://url/b.jpg\" alt=\"Image\"></p>",
            ),
            (
                "> Blockquote",
                "<blockquote>\n<p>Blockquote</p>\n</blockquote>",
            ),
            (
                concat!(
                    "* List\n", //
                    "* List\n", //
                    "* List"
                ),
                "<ul>\n<li>List</li>\n<li>List</li>\n<li>List</li>\n</ul>",
            ),
            (
                concat!(
                    "- List\n", //
                    "- List\n", //
                    "- List"
                ),
                "<ul>\n<li>List</li>\n<li>List</li>\n<li>List</li>\n</ul>",
            ),
            (
                concat!(
                    "1. One\n", //
                    "2. Two\n", //
                    "3. Three"
                ),
                "<ol>\n<li>One</li>\n<li>Two</li>\n<li>Three</li>\n</ol>",
            ),
            (
                concat!(
                    "1) One\n", //
                    "2) Two\n", //
                    "3) Three"
                ),
                "<ol>\n<li>One</li>\n<li>Two</li>\n<li>Three</li>\n</ol>",
            ),
            ("---", "<hr>"),
            ("***", "<hr>"),
            (
                "`Inline code` with backticks",
                "<p><code>Inline code</code> with backticks</p>",
            ),
            (
                concat!(
                    "```\n",                     //
                    "# code block\n",            //
                    "print '3 backticks or'\n",  //
                    "print 'indent 4 spaces'\n", //
                    "```"
                ),
                "<pre><code># code block\nprint '3 backticks or'\nprint 'indent 4 \
                 spaces'\n</code></pre>",
            ),
            (
                concat!(
                    "    # code block\n",           //
                    "    print '3 backticks or'\n", //
                    "    print 'indent 4 spaces'"
                ),
                "<pre><code># code block\nprint '3 backticks or'\nprint 'indent 4 \
                 spaces'\n</code></pre>",
            ),
        ];

        let parser = super::Parser::new();

        for (input, expected) in CASES {
            let result = parser.parse(input);
            assert_eq!(
                result.trim().to_owned(),
                expected.to_owned(),
                "\nparse({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
