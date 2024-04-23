//! Compile TypeScript code.
//!
//! This module uses [`swc_core`] under the hood.

use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum TypescriptError {
    /// Unicode error.
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// TypeScript parser error.
    #[error("{0}")]
    Parser(String),
}

/// Compile TypeScript to JavaScript.
pub fn compile_typescript<S>(input: S, tsx: bool) -> Result<String, TypescriptError>
where
    S: AsRef<str>,
{
    // https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples
    // /ts_to_js.rs
    use swc_core::{
        common::{
            comments::SingleThreadedComments, errors::Handler, sync::Lrc, FileName, Mark,
            SourceMap, GLOBALS,
        },
        ecma::{
            codegen::{text_writer::JsWriter, Emitter},
            parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig},
            transforms::{
                base::{fixer::fixer, hygiene::hygiene, resolver},
                typescript::strip,
            },
            visit::FoldWith,
        },
    };

    let input = input.as_ref();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_emitter_writer(Box::new(std::io::stderr()), Some(cm.clone()));

    let fm = cm.new_source_file(FileName::Anon, input.into());

    let comments = SingleThreadedComments::default();

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
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
        .map_err(|error| TypescriptError::Parser(error.into_diagnostic(&handler).message()))?;

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

            emitter.emit_program(&program)?;
        }

        let result = String::from_utf8(buf)?;

        Ok(result)
    })
}

/// Pipeline task.
pub mod task {
    use super::{compile_typescript, TypescriptError};
    use crate::{
        build::Script,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to compile TypeScript code.
    #[derive(Debug, Default)]
    pub struct TypescriptTask;

    impl TypescriptTask {
        /// Create a pipeline task to compile TypeScript code.
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Task<(Script,), (Script,), TypescriptError> for TypescriptTask {
        fn process(
            self,
            (rx,): (Receiver<Script>,),
            (tx,): (Sender<Script>,),
        ) -> Result<(), TypescriptError> {
            for script in rx {
                let script = if script
                    .input_path
                    .extension()
                    .is_some_and(|extension| extension == "ts")
                {
                    let content = compile_typescript(
                        script.content,
                        script.input_path.extension().is_some_and(|extension| {
                            ["jsx", "tsx"]
                                .map(std::ffi::OsString::from)
                                .contains(&extension.to_os_string())
                        }),
                    )?;

                    Script { content, ..script }
                } else {
                    script
                };

                tx.send(script).unwrap();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compile_typescript;

    #[test]
    fn compile() {
        let result = compile_typescript("const s: string = \"abc\";", false).unwrap();
        assert!(result.contains("abc"));
        assert!(!result.contains("string"));
    }
}
