//! Bundle JavaScript code.
//!
//! This module uses [`swc_core`] under the hood.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

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
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
}

/// Bundle a JavaScript file.
pub fn bundle_js_file(path: impl AsRef<Path>, minify: bool) -> Result<String, BundleJsError> {
    use swc_core::{
        base::{
            config::{JsMinifyOptions, JscConfig},
            resolver::environment_resolver,
            try_with_handler, BoolOrDataConfig, Compiler, PrintArgs,
        },
        bundler::{node::loaders::swc::SwcLoader, Bundler},
        common::{FileName, Globals, SourceMap, GLOBALS},
        ecma::{loader::TargetEnv, transforms::base::fixer::fixer, visit::FoldWith},
    };

    let path = path.as_ref();

    let options = swc_core::base::config::Options {
        filename: path.to_string_lossy().into(),
        config: swc_core::base::config::Config {
            minify: minify.into(),
            jsc: JscConfig {
                minify: Some(JsMinifyOptions {
                    compress: BoolOrDataConfig::from_bool(minify),
                    mangle: BoolOrDataConfig::from_bool(minify),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let entries = HashMap::from([(
        path.to_string_lossy().into(),
        FileName::Real(path.to_path_buf()),
    )]);

    let cm = Arc::new(SourceMap::default());
    let globals = Globals::default();

    let output = GLOBALS.set(&globals, || {
        try_with_handler(cm.clone(), Default::default(), |_handler| {
            let compiler = Arc::new(Compiler::new(cm.clone()));

            let loader = SwcLoader::new(compiler.clone(), options);

            let resolver = environment_resolver(TargetEnv::Browser, Default::default(), false);

            let mut bundler = Bundler::new(
                &globals,
                cm.clone(),
                &loader,
                &resolver,
                swc_core::bundler::Config {
                    require: true,
                    ..Default::default()
                },
                Box::new(Hook),
            );

            let bundles = bundler.bundle(entries)?;

            assert!(bundles.len() == 1);

            let output: String = bundles
                .into_iter()
                .map(move |bundle| {
                    let comments = compiler.comments().clone();

                    let module = bundle
                        .module
                        .fold_with(&mut fixer((!minify).then_some(&comments)));

                    let code = compiler
                        .print(&module, PrintArgs {
                            comments: (!minify).then_some(&comments),
                            codegen_config: swc_core::ecma::codegen::Config::default()
                                .with_minify(minify),
                            ..Default::default()
                        })?
                        .code;

                    Ok(code)
                })
                .collect::<Result<_, anyhow::Error>>()?;

            Ok(output)
        })
    })?;

    Ok(output)
}

/// SWC bundle hook.
pub struct Hook;

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
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to bundle JavaScript code.
    #[derive(Debug, Default)]
    pub struct BundleJsTask {
        minify: bool,
    }

    impl BundleJsTask {
        /// Create a pipeline task to bundle JavaScript code.
        pub fn new(config: &Config) -> Self {
            Self {
                minify: config.optimize,
            }
        }
    }

    impl Task<(Script,), (Script,), BundleJsError> for BundleJsTask {
        fn process(
            self,
            (rx,): (Receiver<Script>,),
            (tx,): (Sender<Script>,),
        ) -> Result<(), BundleJsError> {
            for script in rx {
                let content =
                    bundle_js_file(&script.input_path, self.minify).map_err(|source| {
                        BundleJsError::WithFile {
                            source: Box::new(source),
                            path: script.input_path.clone(),
                        }
                    })?;
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

        std::fs::write(
            &path,
            r#"
                import bar from "./bar.js";
                console.log(bar);
            "#,
        )
        .expect("failed to create file");

        std::fs::write(
            dir.join("bar.js"),
            r#"
                export default "Hello world";
            "#,
        )
        .expect("failed to create file");

        let result = bundle_js_file(path, true).unwrap();

        assert!(result.contains("console.log"));
        assert!(result.contains("Hello world"));
        assert!(!result.contains("bar"));
    }

    #[test]
    fn bundle_typescript_file() {
        let temp_dir = TempDir::new();
        let dir = temp_dir.path();
        let path = dir.join("foo.ts");

        std::fs::write(
            &path,
            r#"
                import bar from "./bar.ts";
                const baz: number = 123;
                console.log(bar);
                console.log(baz);
            "#,
        )
        .expect("failed to create file");

        std::fs::write(
            dir.join("bar.ts"),
            r#"
                const foo: string = "Hello world";
                export default foo;
            "#,
        )
        .expect("failed to create file");

        let result = bundle_js_file(path, true).unwrap();

        assert!(result.contains("console.log"));
        assert!(result.contains("123"));
        assert!(result.contains("Hello world"));

        assert!(!result.contains("bar"));
        assert!(!result.contains("number"));
        assert!(!result.contains("string"));
    }

    /// Requires `npm`.
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
            r#"
                import hi from "hello-world-typescript";
                hi("world");
            "#,
        )
        .expect("failed to create file");

        let result = bundle_js_file(path, true).unwrap();

        assert!(result.contains("console.log"));
        assert!(result.contains("Hello "));
        assert!(!result.contains("hello-world-typescript"));
    }
}
