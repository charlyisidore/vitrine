//! Syntax highlight plugin for Markdown.

use std::collections::HashMap;

use markdown_it::{
    parser::{core::CoreRule, extset::MarkdownItExt},
    plugins::cmark::block::{code::CodeBlock, fence::CodeFence},
    MarkdownIt, Node, NodeValue, Renderer,
};

use crate::{
    build::syntax_highlight::highlight,
    config::{Config, SyntaxHighlightConfig},
    util::function::Function,
};

/// Custom syntax highlighter.
pub type SyntaxHighlighter = Function<(String, HashMap<String, String>), Option<String>>;

/// Add a Markdown rule for syntax highlighting.
pub fn add(md: &mut MarkdownIt, config: &Config) {
    md.add_rule::<SyntaxHighlightRule>();
    md.ext.insert(config.syntax_highlight.clone());
}

// Allow using `md.ext.insert()` to store `SyntaxHighlightConfig`.
impl MarkdownItExt for SyntaxHighlightConfig {}

/// Syntax highlight rule for Markdown.
struct SyntaxHighlightRule;

impl CoreRule for SyntaxHighlightRule {
    fn run(root: &mut Node, md: &MarkdownIt) {
        let config = &md
            .ext
            .get::<SyntaxHighlightConfig>()
            .expect("`syntax_highlight` configuration not found");

        let prefix = if config.css_prefix.is_empty() {
            None
        } else {
            Some(&config.css_prefix)
        };

        root.walk_mut(|node, _| {
            // Detect if the node is a code block or a code fence
            let (content, language) = if let Some(code_block) = node.cast::<CodeBlock>() {
                //     {code_block.content}
                (&code_block.content, None)
            } else if let Some(code_fence) = node.cast::<CodeFence>() {
                // ```{code_fence.info}
                // {code_fence.content}
                // ````
                (&code_fence.content, Some(&code_fence.info))
            } else {
                return;
            };

            let result = config
                .highlighter
                .as_ref()
                .and_then(|function| {
                    let mut attributes = HashMap::new();
                    if let Some(language) = language {
                        attributes.insert("language".to_string(), language.clone());
                    }
                    if let Some(prefix) = &prefix {
                        attributes.insert("prefix".to_string(), prefix.to_string());
                    }
                    function.call(content.clone(), attributes).transpose()
                })
                .transpose();

            // TODO: spread error when markdown_it supports it
            if let Some(error) = result.as_ref().err() {
                eprintln!("markdown::syntax_highlight: {}", error);
                return;
            }

            if let Some(content) = result.unwrap() {
                node.replace(CustomSyntaxHighlight { content });
                return;
            }

            // TODO: spread error when markdown_it supports it
            let Some(content) = highlight(
                content,
                language.map(|s| s.as_str()),
                prefix.map(|s| s.as_str()),
            )
            .inspect_err(|error| {
                eprintln!("markdown::syntax_highlight: {}", error);
            })
            .ok() else {
                return;
            };

            node.replace(BuiltinSyntaxHighlight {
                content,
                language: language.cloned(),
                prefix: config.css_prefix.clone(),
                code_attributes: config.code_attributes.clone(),
                pre_attributes: config.pre_attributes.clone(),
            });
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
