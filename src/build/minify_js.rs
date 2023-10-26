//! Minify JavaScript code.
//!
//! This module uses [`minify_js`] under the hood.

use minify_js::{Session, TopLevelMode};

use super::{Entry, Error};

/// Minify JavaScript content of a [`Entry`].
///
/// This function minifies JavaScript code in the `content` property.
pub(super) fn minify_entry(entry: Entry) -> Result<Entry, Error> {
    if let Some(content) = entry.content.as_ref() {
        let content = minify(content).map_err(|error| Error::MinifyJs {
            input_path: entry.input_path_buf(),
            source: error,
        })?;

        return Ok(Entry {
            content: Some(content),
            ..entry
        });
    }

    Ok(entry)
}

/// Minify a string containing JavaScript code.
fn minify<S>(input: S) -> Result<String, anyhow::Error>
where
    S: AsRef<str>,
{
    let input = input.as_ref();

    let session = Session::new();
    let mut output = Vec::new();

    minify_js::minify(
        &session,
        TopLevelMode::Global,
        input.as_bytes(),
        &mut output,
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let output = String::from_utf8(output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                "function hello() {\n",      //
                "  console.log('hello');\n", //
                "}"
            ),
            "var hello=(()=>{console.log(`hello`)})",
        )];

        for (input, expected) in CASES {
            let result = super::minify(input).unwrap();
            assert_eq!(
                result,
                expected.to_owned(),
                "\nminify({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
