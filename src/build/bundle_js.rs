//! Bundle JavaScript code.
//!
//! This module uses [`swc_core`] under the hood.

use std::{collections::HashMap, path::Path};

use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum BundleJsError {
    /// Anyhow error.
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Bundle a JavaScript file.
pub fn bundle_js_file(path: impl AsRef<Path>) -> Result<String, BundleJsError> {
    use swc_core::{
        bundler::Bundler,
        common::{sync::Lrc, FileName, FilePathMapping, SourceMap},
        ecma::{
            codegen::{text_writer::JsWriter, Emitter},
            loader::{
                resolvers::{lru::CachingResolver, node::NodeModulesResolver},
                TargetEnv,
            },
        },
    };

    let path = path.as_ref();

    const MINIFY: bool = false;
    const INLINE: bool = true;

    let entries = HashMap::from([("main".to_string(), FileName::Real(path.to_path_buf()))]);

    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let globals = Box::leak(Box::default());
    let mut bundler = Bundler::new(
        globals,
        cm.clone(),
        Loader { cm: cm.clone() },
        CachingResolver::new(
            4096,
            NodeModulesResolver::new(TargetEnv::Node, Default::default(), true),
        ),
        swc_core::bundler::Config {
            require: true,
            disable_inliner: !INLINE,
            external_modules: Default::default(),
            disable_fixer: MINIFY,
            disable_hygiene: MINIFY,
            disable_dce: false,
            module: Default::default(),
        },
        Box::new(Hook),
    );

    let modules = bundler.bundle(entries)?;

    Ok(modules
        .into_iter()
        .map(|bundled| {
            let mut buf = Vec::new();

            {
                let wr = JsWriter::new(cm.clone(), "\n", &mut buf, None);
                let mut emitter = Emitter {
                    cfg: swc_core::ecma::codegen::Config::default().with_minify(MINIFY),
                    cm: cm.clone(),
                    comments: None,
                    wr: Box::new(wr),
                };

                emitter.emit_module(&bundled.module)?;
            }

            Ok(String::from_utf8_lossy(&buf).to_string())
        })
        .collect::<Result<Vec<_>, BundleJsError>>()?
        .join("\n"))
}

struct Loader {
    cm: swc_core::common::sync::Lrc<swc_core::common::SourceMap>,
}

impl swc_core::bundler::Load for Loader {
    fn load(
        &self,
        f: &swc_core::common::FileName,
    ) -> Result<swc_core::bundler::ModuleData, anyhow::Error> {
        use swc_core::{
            bundler::ModuleData,
            common::{errors::Handler, FileName},
            ecma::{
                ast::EsVersion,
                parser::{parse_file_as_module, Syntax},
            },
        };

        let fm = match f {
            FileName::Real(path) => {
                let extension = path.extension();
                if extension.is_some_and(|extension| {
                    ["ts", "tsx"]
                        .map(Into::into)
                        .contains(&extension.to_os_string())
                }) {
                    let tsx = extension.is_some_and(|extension| extension == "tsx");
                    let source = std::fs::read_to_string(path)?;
                    let source = crate::build::typescript::compile_typescript(source, tsx)?;
                    self.cm.new_source_file(f.clone(), source)
                } else {
                    self.cm.load_file(path)?
                }
            },
            _ => unreachable!(),
        };

        let module = parse_file_as_module(
            &fm,
            Syntax::Es(Default::default()),
            EsVersion::Es2020,
            None,
            &mut vec![],
        )
        .unwrap_or_else(|err| {
            let handler =
                Handler::with_emitter_writer(Box::new(std::io::stderr()), Some(self.cm.clone()));
            err.into_diagnostic(&handler).emit();
            panic!("failed to parse")
        });

        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}

struct Hook;

impl swc_core::bundler::Hook for Hook {
    fn get_import_meta_props(
        &self,
        span: swc_core::common::Span,
        module_record: &swc_core::bundler::ModuleRecord,
    ) -> Result<Vec<swc_core::ecma::ast::KeyValueProp>, anyhow::Error> {
        use swc_core::ecma::ast::{
            Bool, Expr, Ident, KeyValueProp, Lit, MemberExpr, MemberProp, MetaPropExpr,
            MetaPropKind, PropName, Str,
        };

        let file_name = module_record.file_name.to_string();

        Ok(vec![
            KeyValueProp {
                key: PropName::Ident(Ident::new("url".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: file_name.into(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(Ident::new("main".into(), span)),
                value: Box::new(if module_record.is_entry {
                    Expr::Member(MemberExpr {
                        span,
                        obj: Box::new(Expr::MetaProp(MetaPropExpr {
                            span,
                            kind: MetaPropKind::ImportMeta,
                        })),
                        prop: MemberProp::Ident(Ident::new("main".into(), span)),
                    })
                } else {
                    Expr::Lit(Lit::Bool(Bool { span, value: false }))
                }),
            },
        ])
    }
}

/// Pipeline task.
pub mod task {
    use super::{bundle_js_file, BundleJsError};
    use crate::{
        build::Script,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to bundle JavaScript code.
    #[derive(Debug, Default)]
    pub struct BundleJsTask;

    impl BundleJsTask {
        /// Create a pipeline task to bundle JavaScript code.
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Task<(Script,), (Script,), BundleJsError> for BundleJsTask {
        fn process(
            self,
            (rx,): (Receiver<Script>,),
            (tx,): (Sender<Script>,),
        ) -> Result<(), BundleJsError> {
            for script in rx {
                let content = bundle_js_file(&script.input_path)?;
                tx.send(Script { content, ..script }).unwrap();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::bundle_js_file;
    use crate::util::temp_dir::TempDir;

    #[test]
    fn bundle_javascript_file() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();
        let path = dir.join("foo.js");

        std::fs::write(&path, "import bar from \"./bar.js\";\nconsole.log(bar);")
            .expect("failed to create file");

        std::fs::write(dir.join("bar.js"), r#"export default "Hello world";"#)
            .expect("failed to create file");

        let result = bundle_js_file(path).unwrap();

        assert!(result.contains("Hello world"));
    }

    #[test]
    fn bundle_typescript_file() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();
        let path = dir.join("foo.ts");

        std::fs::write(
            &path,
            "import bar from \"./bar.ts\";\nconst baz: number = \
             123;\nconsole.log(bar);\nconsole.log(baz);",
        )
        .expect("failed to create file");

        std::fs::write(
            dir.join("bar.ts"),
            "const foo: string = \"Hello world\";\nexport default foo;",
        )
        .expect("failed to create file");

        let result = bundle_js_file(path).unwrap();

        assert!(result.contains("Hello world"));
        assert!(result.contains("123"));
        assert!(!result.contains("string"));
        assert!(!result.contains("number"));
    }

    #[cfg(any())]
    #[test]
    fn bundle_with_npm() {
        use std::process::Command;

        let temp_dir = TempDir::new();
        let dir = temp_dir.path();

        Command::new("npm")
            .args(["init", "-y"])
            .current_dir(&dir)
            .status()
            .expect("failed to execute `npm init` command");

        Command::new("npm")
            .args(["install", "hello-world-typescript@1.0.1"])
            .current_dir(&dir)
            .status()
            .expect("failed to execute `npm install` command");

        let path = dir.join("index.js");

        std::fs::write(
            &path,
            "import hi from \"hello-world-typescript\";\nhi(\"world\");",
        )
        .expect("failed to create file");

        let result = bundle_js_file(path).unwrap();

        assert!(result.contains("console.log"));
        assert!(result.contains("Hello "));
    }
}
