//! Syntax highlight plugin for Markdown.
//!
//! This module uses [`syntect`] under the hood.

use markdown_it::{
    parser::core::CoreRule,
    plugins::cmark::block::{code::CodeBlock, fence::CodeFence},
    MarkdownIt, Node, NodeValue, Renderer,
};
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use super::Context;

/// Add a Markdown rule for syntax highlighting.
pub(super) fn add(md: &mut MarkdownIt) {
    md.add_rule::<HighlightRule>();
}

/// Syntax highlight rule for Markdown.
struct HighlightRule;

impl CoreRule for HighlightRule {
    fn run(root: &mut Node, md: &MarkdownIt) {
        let context = md.ext.get::<Context>().unwrap();
        dbg!(&context);

        // Since [`syntect`]` requires `'static` lifetime for `prefix` in
        // [`syntect::html::ClassStyle::SpacedPrefixed`], we cannot use a value created
        // at runtime. Therefore, we use `static_lifetime()` as a workaround.
        let prefix =
            unsafe { crate::util::r#unsafe::static_lifetime(&context.syntax_highlight_css_prefix) };

        let syntax_set = SyntaxSet::load_defaults_newlines();

        root.walk_mut(|node, _| {
            let (content, language) = if let Some(code_block) = node.cast::<CodeBlock>() {
                //     {code_block.content}
                (Some(&code_block.content), None)
            } else if let Some(code_fence) = node.cast::<CodeFence>() {
                // ```{code_fence.info}
                // {code_fence.content}
                // ````
                (Some(&code_fence.content), Some(&code_fence.info))
            } else {
                (None, None)
            };

            if let Some(content) = content {
                let syntax = language
                    .and_then(|language| syntax_set.find_syntax_by_token(language))
                    .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

                let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
                    syntax,
                    &syntax_set,
                    ClassStyle::SpacedPrefixed { prefix },
                );

                for line in LinesWithEndings::from(content) {
                    html_generator
                        .parse_html_for_line_which_includes_newline(line)
                        .unwrap_or_else(|error| {
                            tracing::error!("markdown::syntax_highlight: {}", error)
                        });
                }

                let content = html_generator.finalize();

                node.replace(Highlight {
                    content,
                    language: language.map(|s| s.to_owned()),
                    prefix: prefix.to_owned(),
                });
            }
        });
    }
}

/// AST node for the syntax highlight Markdown rule.
#[derive(Debug)]
struct Highlight {
    content: String,
    language: Option<String>,
    prefix: String,
}

impl NodeValue for Highlight {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        const PRE: &str = "pre";
        const CODE: &str = "code";
        if let Some(language) = self.language.as_ref().filter(|v| !v.is_empty()) {
            let attrs = [(
                "class",
                format!("{}code language-{}", self.prefix, &language),
            )];
            fmt.open(PRE, &attrs);
            fmt.open(CODE, &attrs);
        } else {
            let attrs = [("class", format!("{}code", self.prefix))];
            fmt.open(PRE, &attrs);
            fmt.open(CODE, &attrs);
        }
        fmt.text_raw(&self.content);
        fmt.close(CODE);
        fmt.close(PRE);
    }
}
