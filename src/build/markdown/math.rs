//! Math plugin for Markdown.
//!
//! This module uses [`katex`] under the hood.

use markdown_it::{
    parser::{
        block::{BlockRule, BlockState},
        inline::{InlineRule, InlineState},
    },
    MarkdownIt, Node, NodeValue, Renderer,
};

/// Add a Markdown rule for rendering math in LaTeX.
pub fn add(md: &mut MarkdownIt) {
    md.inline.add_rule::<MathInlineRule>();
    md.block.add_rule::<MathBlockRule>();
}

/// Render math in `<eq>...</eq>`.
#[derive(Debug)]
struct MathInline(String);

impl NodeValue for MathInline {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        const EQ: &str = "eq";
        fmt.open(EQ, &[]);
        fmt.text_raw(&self.0);
        fmt.close(EQ);
    }
}

/// Render math in `<section><eqn>...</eqn></section>`.
#[derive(Debug)]
struct MathBlock(String);

impl NodeValue for MathBlock {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        const SECTION: &str = "section";
        const EQN: &str = "eqn";
        fmt.open(SECTION, &[]);
        fmt.open(EQN, &[]);
        fmt.text_raw(&self.0);
        fmt.close(SECTION);
        fmt.close(EQN);
    }
}

/// Math inline rule for Markdown.
pub struct MathInlineRule;

impl InlineRule for MathInlineRule {
    const MARKER: char = '$';

    fn run(state: &mut InlineState) -> Option<(Node, usize)> {
        let input = &state.src[state.pos..state.pos_max];

        if !input.starts_with(Self::MARKER) {
            return None;
        }

        let Some(length) = input[1..].find(Self::MARKER).filter(|&length| length > 0) else {
            return None;
        };

        let content = &input[1..length + 1];

        let result = katex::render(content);

        if let Some(error) = result.as_ref().err() {
            tracing::error!("markdown::math: {}", error);
        }

        result
            .ok()
            .map(|html| (Node::new(MathInline(html)), length + 2))
    }
}

/// Math block rule for Markdown.
pub struct MathBlockRule;

impl BlockRule for MathBlockRule {
    fn run(state: &mut BlockState) -> Option<(Node, usize)> {
        const DELIMITER: &str = "$$";

        let line = state.get_line(state.line).trim();

        if line.len() <= 2 * DELIMITER.len()
            || !line.starts_with(DELIMITER)
            || !line.ends_with(DELIMITER)
        {
            return None;
        }

        let content = &line[DELIMITER.len()..line.len() - DELIMITER.len()];

        let opts = katex::Opts::builder().display_mode(true).build();

        if let Some(error) = opts.as_ref().err() {
            tracing::error!("markdown::math: {}", error);
            return None;
        }

        let result = katex::render_with_opts(content, opts.unwrap());

        if let Some(error) = result.as_ref().err() {
            tracing::error!("markdown::math: {}", error);
        }

        result.ok().map(|html| (Node::new(MathBlock(html)), 1))
    }
}
