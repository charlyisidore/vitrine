//! Minify JavaScript code.
//!
//! This module uses [`swc_core`] under the hood.

use std::sync::Arc;

use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MinifyJsError {
    /// Anyhow error.
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

/// Minify a string of JavaScript code.
pub fn minify_js(input: impl AsRef<str>) -> Result<String, MinifyJsError> {
    use swc_core::{
        base::{
            config::{JsMinifyOptions, JscConfig},
            try_with_handler, BoolOrDataConfig, Compiler,
        },
        common::{FileName, SourceMap, GLOBALS},
    };

    let input = input.as_ref();

    let options = swc_core::base::config::Options {
        config: swc_core::base::config::Config {
            minify: true.into(),
            jsc: JscConfig {
                minify: Some(JsMinifyOptions {
                    compress: BoolOrDataConfig::from_bool(true),
                    mangle: BoolOrDataConfig::from_bool(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let cm = Arc::new(SourceMap::default());
    let compiler = Compiler::new(cm.clone());

    let output = GLOBALS.set(&Default::default(), || {
        try_with_handler(cm.clone(), Default::default(), |handler| {
            let fm = cm.new_source_file(FileName::Anon.into(), input.to_string());
            let output = compiler.process_js_file(fm, handler, &options)?;
            Ok(output.code)
        })
    })?;

    Ok(output)
}

/// Pipeline task.
pub mod task {
    use super::{minify_js, MinifyJsError};
    use crate::{
        build::Script,
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to minify JavaScript code.
    #[derive(Debug)]
    pub struct MinifyJsTask<'config> {
        config: &'config Config,
    }

    impl<'config> MinifyJsTask<'config> {
        /// Create a pipeline task to minify JavaScript code.
        pub fn new(config: &'config Config) -> Self {
            Self { config }
        }
    }

    impl Task<(Script,), (Script,), MinifyJsError> for MinifyJsTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Script>,),
            (tx,): (Sender<Script>,),
        ) -> Result<(), MinifyJsError> {
            for script in rx {
                let script = if self.config.optimize {
                    let content = minify_js(script.content)?;
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
    use super::minify_js;

    #[test]
    fn minify() {
        // Length: 177
        const INPUT: &str = r#"
            // Lorem ipsum dolor sit amet
            document.addEventListener("DOMContentLoaded", () => {
                console.log("Hello world");
            });
        "#;

        let result = minify_js(INPUT).unwrap();

        assert!(result.contains("document.addEventListener"));
        assert!(result.contains("DOMContentLoaded"));
        assert!(result.contains("console.log"));
        assert!(result.contains("Hello world"));

        assert!(!result.contains("Lorem"));
        assert!(!result.contains('\n'));

        // Expected: 85
        assert!(result.len() <= 90);
    }
}
