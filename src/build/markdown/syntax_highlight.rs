//! Syntax highlight plugin for Markdown.
//!
//! This module uses [`syntect`] under the hood.

use std::collections::HashMap;

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
    md.add_rule::<SyntaxHighlightRule>();
}

/// Syntax highlight rule for Markdown.
struct SyntaxHighlightRule;

impl CoreRule for SyntaxHighlightRule {
    fn run(root: &mut Node, md: &MarkdownIt) {
        let context = &md.ext.get::<Context>().unwrap().syntax_highlight;

        // Since [`syntect`]` requires `'static` lifetime for `prefix` in
        // [`syntect::html::ClassStyle::SpacedPrefixed`], we cannot use a value created
        // at runtime. Therefore, we use `static_lifetime()` as a workaround.
        let prefix = unsafe { crate::util::r#unsafe::static_lifetime(&context.css_prefix) };

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
                let result = context
                    .formatter
                    .as_ref()
                    .and_then(|function| {
                        let mut attributes = HashMap::new();
                        if let Some(language) = language {
                            attributes.insert("language".to_owned(), language.to_owned());
                        }
                        function.call_2(content, &attributes).transpose()
                    })
                    .transpose();

                if let Some(error) = result.as_ref().err() {
                    tracing::error!("markdown::syntax_highlight: {}", error);
                    return;
                }

                if let Some(content) = result.unwrap() {
                    node.replace(CustomSyntaxHighlight { content });
                    return;
                }

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

                node.replace(BuiltinSyntaxHighlight {
                    content,
                    language: language.map(|s| s.to_owned()),
                    prefix: prefix.to_owned(),
                    code_attributes: context.code_attributes.to_owned(),
                    pre_attributes: context.pre_attributes.to_owned(),
                });
            }
        });
    }
}

/// AST node for builtin syntax highlight.
#[derive(Debug)]
struct BuiltinSyntaxHighlight {
    content: String,
    language: Option<String>,
    prefix: String,
    code_attributes: HashMap<String, String>,
    pre_attributes: HashMap<String, String>,
}

impl NodeValue for BuiltinSyntaxHighlight {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        const PRE: &str = "pre";
        const CODE: &str = "code";

        let mut code_attributes = self.code_attributes.clone();
        let mut pre_attributes = self.pre_attributes.clone();

        let class = if let Some(language) = self.language.as_ref().filter(|v| !v.is_empty()) {
            format!("{}code language-{}", self.prefix, &language)
        } else {
            format!("{}code", self.prefix)
        };

        code_attributes
            .entry("class".to_owned())
            .or_insert_with(|| class.to_owned());

        pre_attributes
            .entry("class".to_owned())
            .or_insert_with(|| class.to_owned());

        let code_attributes: Vec<_> = code_attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_owned()))
            .collect();

        let pre_attributes: Vec<_> = pre_attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_owned()))
            .collect();

        fmt.open(PRE, &pre_attributes);
        fmt.open(CODE, &code_attributes);
        fmt.text_raw(&self.content);
        fmt.close(CODE);
        fmt.close(PRE);
    }
}

/// AST node for custom syntax highlight.
#[derive(Debug)]
struct CustomSyntaxHighlight {
    content: String,
}

impl NodeValue for CustomSyntaxHighlight {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        fmt.text_raw(&self.content);
    }
}
