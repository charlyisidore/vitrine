//! Katex plugin for Markdown.

use anyhow::{Result, anyhow};
use async_channel::{Receiver, Sender, unbounded};
use markdown_it::{
    MarkdownIt, Node, NodeValue, Renderer,
    parser::{
        block::{BlockRule, BlockState},
        extset::RootExt,
        inline::{InlineRule, InlineState},
    },
};
use vitrine_deno::deno_runtime::deno_core::v8;

use crate::ReceiverExt;

/// Add a Markdown rule for rendering KaTeX expressions.
pub fn add(md: &mut MarkdownIt) {
    md.inline.add_rule::<KatexInlineRule>();
    md.block.add_rule::<KatexBlockRule>();
}

/// KaTeX runtime.
#[derive(Debug)]
struct KatexRuntime {
    tx: Sender<String>,
    rx: Receiver<Result<String>>,
}

impl KatexRuntime {
    /// Create a KaTeX runtime.
    pub fn new() -> Self {
        let (tx_in, rx_in) = unbounded();
        let (tx_out, rx_out) = unbounded();
        std::thread::spawn(|| Self::run(rx_in, tx_out));
        Self {
            tx: tx_in,
            rx: rx_out,
        }
    }

    /// Render a KaTeX expression into a string.
    pub fn render_to_string(&self, input: String) -> Result<String> {
        self.tx.send_blocking(input)?;
        self.rx.recv_blocking()?
    }

    /// Run v8 thread.
    fn run(rx: Receiver<String>, tx: Sender<Result<String>>) -> Result<()> {
        let isolate = &mut v8::Isolate::new(Default::default());
        let scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let code = v8::String::new(scope, include_str!("katex.mjs")).unwrap();

        let origin = v8::undefined(scope);
        let origin = v8::ScriptOrigin::new(
            scope,
            origin.into(),
            0,
            0,
            false,
            0,
            None,
            false,
            false,
            true,
            None,
        );

        let mut source = v8::script_compiler::Source::new(code, Some(&origin));

        let module = v8::script_compiler::compile_module(scope, &mut source).unwrap();
        module.instantiate_module(scope, |_, _, _, _| None).unwrap();
        module.evaluate(scope).unwrap();

        let module = module.get_module_namespace();
        let module = module.to_object(scope).unwrap();

        let render_to_string = v8::String::new(scope, "renderToString").unwrap();
        let render_to_string = module.get(scope, render_to_string.into()).unwrap();
        let render_to_string = render_to_string.try_cast::<v8::Function>()?;

        for input in rx.into_iter() {
            let scope = &mut v8::TryCatch::new(scope);
            let this = v8::undefined(scope);
            let args = &[v8::String::new(scope, &input).unwrap().into()];
            let result = render_to_string
                .call(scope, this.into(), args)
                .map(|value| value.to_rust_string_lossy(scope))
                .ok_or_else(|| {
                    if scope.has_caught() {
                        anyhow!(scope.exception().unwrap().to_rust_string_lossy(scope))
                    } else {
                        anyhow!("unknown error")
                    }
                });
            tx.send_blocking(result)?;
        }

        Ok(())
    }
}

impl RootExt for KatexRuntime {}

/// Render KaTeX in `<eq>...</eq>`.
#[derive(Debug)]
struct KatexInline(String);

impl NodeValue for KatexInline {
    fn render(&self, _: &Node, fmt: &mut dyn Renderer) {
        const EQ: &str = "eq";
        fmt.open(EQ, &[]);
        fmt.text_raw(&self.0);
        fmt.close(EQ);
    }
}

/// Render KaTeX in `<section><eqn>...</eqn></section>`.
#[derive(Debug)]
struct KatexBlock(String);

impl NodeValue for KatexBlock {
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

/// KaTeX inline rule for Markdown.
pub struct KatexInlineRule;

impl InlineRule for KatexInlineRule {
    const MARKER: char = '$';

    fn run(state: &mut InlineState) -> Option<(Node, usize)> {
        let input = &state.src[state.pos..state.pos_max];

        if !input.starts_with(Self::MARKER) {
            return None;
        }

        let length = input[1..].find(Self::MARKER).filter(|&length| length > 0)?;

        let content = input[1..length + 1].to_string();

        let runtime = state.root_ext.get_or_insert_with(KatexRuntime::new);
        let result = runtime.render_to_string(content);

        if let Err(error) = result.as_ref() {
            log::error!("markdown::katex: {:?}", error);
        }

        result
            .ok()
            .map(|html| (Node::new(KatexInline(html)), length + 2))
    }
}

/// KaTeX block rule for Markdown.
pub struct KatexBlockRule;

impl BlockRule for KatexBlockRule {
    fn run(state: &mut BlockState) -> Option<(Node, usize)> {
        const DELIMITER: &str = "$$";

        let line = state.get_line(state.line).trim();

        if line.len() <= 2 * DELIMITER.len()
            || !line.starts_with(DELIMITER)
            || !line.ends_with(DELIMITER)
        {
            return None;
        }

        let content = line[DELIMITER.len()..line.len() - DELIMITER.len()].to_string();

        let runtime = state.root_ext.get_or_insert_with(KatexRuntime::new);
        let result = runtime.render_to_string(content);

        if let Err(error) = result.as_ref() {
            log::error!("markdown::katex: {:?}", error);
        }

        result.ok().map(|html| (Node::new(KatexBlock(html)), 1))
    }
}
