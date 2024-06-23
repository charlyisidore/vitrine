//! Compile TypeScript code.
//!
//! This module uses [`swc_core`] under the hood.

use swc_core::{
    common::{
        comments::SingleThreadedComments, errors::Handler, sync::Lrc, FileName, Mark, SourceMap,
        GLOBALS,
    },
    ecma::{
        codegen::{text_writer::JsWriter, Emitter},
        parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax},
        transforms::{
            base::{fixer::fixer, hygiene::hygiene, resolver},
            typescript::strip,
        },
        visit::FoldWith,
    },
};

use super::{Entry, Error};

/// Compile TypeScript content of a [`Entry`] to JavaScript content.
///
/// This function transpiles the TypeScript code to JavaScript in the `content`
/// property. The `format` property is set to `js`.
pub(super) fn compile_entry(entry: Entry) -> Result<Entry, Error> {
    let Some(content) = entry.content else {
        return Ok(entry);
    };

    let tsx = match entry.format.as_str() {
        "tsx" => true,
        _ => false,
    };

    let content = compile(content, tsx).map_err(|error| Error::CompileTypescript {
        input_path: entry
            .input_file
            .as_ref()
            .map(|entry| entry.path().to_owned()),
        source: error,
    })?;

    // Change extension to `js`
    let url = entry
        .url
        .rsplit_once('.')
        .filter(|(_, extension)| ["js", "jsx", "ts", "tsx"].contains(extension))
        .map(|(stem, _)| [stem, "js"].join("."))
        .unwrap_or([entry.url.as_str(), "js"].join("."));

    Ok(Entry {
        content: Some(content),
        format: "js".to_owned(),
        url,
        ..entry
    })
}

/// Compile a TypeScript content to a JavaScript content.
///
/// <https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples
/// /ts_to_js.rs>
fn compile<S>(input: S, tsx: bool) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let input = input.as_ref();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_emitter_writer(Box::new(std::io::stderr()), Some(cm.clone()));

    let fm = cm.new_source_file(FileName::Anon, input.into());

    let comments = SingleThreadedComments::default();

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    for error in parser.take_errors() {
        error.into_diagnostic(&handler).emit();
    }

    let program = parser
        .parse_program()
        .map_err(|error| error.into_diagnostic(&handler).message())
        .map_err(|error| anyhow::anyhow!(error).context("Failed to parse typescript code"))?;

    GLOBALS.set(&Default::default(), || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        // Optionally transforms decorators here before the resolver pass
        // as it might produce runtime declarations.

        // Conduct identifier scope analysis
        let program = program.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

        // Remove typescript types
        let program = program.fold_with(&mut strip(top_level_mark));

        // Fix up any identifiers with the same name, but different contexts
        let program = program.fold_with(&mut hygiene());

        // Ensure that we have enough parenthesis.
        let program = program.fold_with(&mut fixer(Some(&comments)));

        let mut buf = vec![];
        {
            let mut emitter = Emitter {
                cfg: swc_core::ecma::codegen::Config::default(),
                cm: cm.clone(),
                comments: Some(&comments),
                wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };

            emitter
                .emit_program(&program)
                .map_err(|error| anyhow::anyhow!(error))?;
        }

        String::from_utf8(buf).map_err(|error| error.into())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile() {
        const CASES: [(&str, &str); 1] = [("const s: string = \"abc\";", "const s = \"abc\";\n")];

        for (input, expected) in CASES {
            let result = super::compile(input, false).unwrap();
            assert_eq!(
                result,
                expected.to_owned(),
                "\ncompile({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
