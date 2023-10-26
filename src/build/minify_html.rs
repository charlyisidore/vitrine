//! Minify HTML code.
//!
//! This module uses [`minify_html`] under the hood.

use minify_html::{minify, Cfg};

use super::{Entry, Error};

/// HTML minifier.
pub(super) struct Minifier {
    cfg: Cfg,
}

impl Minifier {
    /// Create and configure a HTML minifier.
    pub(super) fn new() -> Self {
        Self {
            cfg: Cfg::spec_compliant(),
        }
    }

    /// Minify HTML content of a [`Entry`].
    ///
    /// This function minifies HTML code in the `content` property.
    pub(super) fn minify_entry(&self, entry: Entry) -> Result<Entry, Error> {
        if let Some(content) = entry.content.as_ref() {
            let content = self.minify(content).map_err(|error| Error::MinifyHtml {
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

    /// Minify a HTML string.
    fn minify<S>(&self, input: S) -> Result<String, anyhow::Error>
    where
        S: AsRef<str>,
    {
        let input = input.as_ref();

        // `minify()` accepts and returns a `Vec<u8>`
        let output = minify(input.as_bytes(), &self.cfg);

        // Convert `Vec<u8>` to `String`
        let output = String::from_utf8(output)?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                "<html>\n",          //
                "  <head></head>\n", //
                "  <body></body>\n", //
                "</html>\n"
            ),
            "<body>",
        )];

        let minifier = super::Minifier::new();

        for (input, expected) in CASES {
            let result = minifier.minify(input).unwrap();
            assert_eq!(
                result,
                expected.to_owned(),
                "\nminify({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
