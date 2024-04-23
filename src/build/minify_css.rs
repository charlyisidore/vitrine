//! Minify CSS code.
//!
//! This module uses [`lightningcss`] under the hood.

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MinifyCssError {
    /// Minifier error.
    #[error("{0}")]
    Minify(String),
    /// Parser error.
    #[error("{0}")]
    Parser(String),
    /// Printer error.
    #[error("{0}")]
    Printer(String),
}

/// Minify a CSS string.
pub fn minify_css(input: impl AsRef<str>) -> Result<String, MinifyCssError> {
    let input = input.as_ref();

    let parser_options = ParserOptions::default();

    let mut style_sheet = StyleSheet::parse(input, parser_options)
        .map_err(|source| MinifyCssError::Parser(source.to_string()))?;

    let minify_options = MinifyOptions::default();

    style_sheet
        .minify(minify_options)
        .map_err(|source| MinifyCssError::Minify(source.to_string()))?;

    let printer_options = PrinterOptions {
        minify: true,
        ..Default::default()
    };

    let result = style_sheet
        .to_css(printer_options)
        .map_err(|source| MinifyCssError::Printer(source.to_string()))?;

    Ok(result.code)
}

/// Pipeline task.
pub mod task {
    use super::{minify_css, MinifyCssError};
    use crate::{
        build::Style,
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to minify CSS code.
    #[derive(Debug)]
    pub struct MinifyCssTask<'config> {
        config: &'config Config,
    }

    impl<'config> MinifyCssTask<'config> {
        /// Create a pipeline task to minify CSS code.
        pub fn new(config: &'config Config) -> Self {
            Self { config }
        }
    }

    impl Task<(Style,), (Style,), MinifyCssError> for MinifyCssTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Style>,),
            (tx,): (Sender<Style>,),
        ) -> Result<(), MinifyCssError> {
            for style in rx {
                let style = if self.config.optimize {
                    let content = minify_css(style.content)?;
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
    use super::minify_css;

    #[test]
    fn minify() {
        // Length: 25
        const INPUT: &str = concat!(
            ".foo {\n",          //
            "  color: black;\n", //
            "}\n"
        );

        let result = minify_css(INPUT).unwrap();

        assert!(result.contains(".foo"));
        assert!(result.contains("color:"));
        // Expected: 16
        assert!(result.len() <= 18);
    }
}
