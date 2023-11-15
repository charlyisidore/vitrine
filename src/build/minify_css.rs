//! Minify CSS code.
//!
//! This module uses [`lightningcss`] under the hood.

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};

use super::{Entry, Error};

/// Minify CSS content in a [`Entry`].
///
/// This function minifies CSS code in the `content` property.
pub(super) fn minify_entry(entry: Entry) -> Result<Entry, Error> {
    if let Some(content) = entry.content.as_ref() {
        let content = minify(content).map_err(|error| Error::MinifyCss {
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

/// Minify a CSS string.
pub(super) fn minify<S>(input: S) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let input = input.as_ref();

    let mut style_sheet = StyleSheet::parse(input, ParserOptions::default())
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    style_sheet.minify(MinifyOptions::default())?;

    let printer_options = PrinterOptions {
        minify: true,
        ..Default::default()
    };

    let result = style_sheet.to_css(printer_options)?;

    Ok(result.code)
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                ".container {\n",    //
                "  color: black;\n", //
                "}"
            ),
            ".container{color:#000}",
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
