//! Compile SCSS code.
//!
//! This module uses [`grass`] under the hood.

use grass::Options;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum CompileScssError {
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
    pub fn compile(&self, input: impl AsRef<str>) -> Result<String, CompileScssError> {
        Ok(grass::from_string(input.as_ref(), &self.options)?)
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
