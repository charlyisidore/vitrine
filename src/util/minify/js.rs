//! Minify JavaScript code.
//!
//! This module uses [`minify_js`] under the hood.

use std::string::FromUtf8Error;

use minify_js::{Session, TopLevelMode};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MinifyJsError {
    /// JavaScript syntax error.
    #[error("{0}")]
    Syntax(String),
    /// Error converting a string in UTF-8.
    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),
}

/// Minify a string of JavaScript code.
pub fn minify_js(input: impl AsRef<str>) -> Result<String, MinifyJsError> {
    let input = input.as_ref();

    let session = Session::new();
    let mut output = Vec::new();

    minify_js::minify(
        &session,
        TopLevelMode::Global,
        input.as_bytes(),
        &mut output,
    )
    .map_err(|source| MinifyJsError::Syntax(source.to_string()))?;

    Ok(String::from_utf8(output)?)
}

#[cfg(test)]
mod tests {
    use super::minify_js;

    #[test]
    fn minify() {
        // Length: 41
        const INPUT: &str = concat!(
            "function foo() {\n",        //
            "  console.log(\"bar\");\n", //
            "}\n"
        );

        let result = minify_js(INPUT).unwrap();

        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
        assert!(!result.contains('\n'));
        // Expected: 34
        assert!(result.len() <= 36);
    }
}
