//! Compile SCSS code.
//!
//! This module uses [`grass`] under the hood.

use grass::Options;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum ScssError {
    /// Grass error.
    #[error(transparent)]
    Grass(#[from] Box<grass::Error>),
}

/// SCSS compiler.
#[derive(Default)]
pub struct ScssCompiler<'o> {
    options: Options<'o>,
}

impl ScssCompiler<'_> {
    /// Create a SCSS compiler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile a SCSS string to CSS.
    pub fn compile(&self, input: impl AsRef<str>) -> Result<String, ScssError> {
        Ok(grass::from_string(input.as_ref(), &self.options)?)
    }
}

/// Pipeline task.
pub mod task {
    use super::{ScssCompiler, ScssError};
    use crate::{
        build::Style,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to compile SCSS code.
    #[derive(Default)]
    pub struct ScssTask<'compiler> {
        compiler: ScssCompiler<'compiler>,
    }

    impl ScssTask<'_> {
        /// Create a pipeline task to compile SCSS code.
        pub fn new() -> Self {
            let compiler = ScssCompiler::new();

            Self { compiler }
        }
    }

    impl Task<(Style,), (Style,), ScssError> for ScssTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Style>,),
            (tx,): (Sender<Style>,),
        ) -> Result<(), ScssError> {
            for style in rx {
                let style = if style
                    .input_path
                    .extension()
                    .is_some_and(|extension| extension == "scss")
                {
                    let content = self.compiler.compile(style.content)?;
                    Style { content, ..style }
                } else {
                    style
                };

                tx.send(style).unwrap();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScssCompiler;

    #[test]
    fn compile_scss() {
        let compiler = ScssCompiler::new();
        let result = compiler.compile(".a { .b { color: #000; } }").unwrap();
        assert!(result.contains(".a .b"));
    }
}
