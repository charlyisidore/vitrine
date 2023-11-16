//! Compile SCSS code.
//!
//! This module uses [`grass`] under the hood.

use grass::Options;

use super::{Entry, Error};

/// SCSS compiler.
pub(super) struct Compiler<'o> {
    options: Options<'o>,
}

impl Compiler<'_> {
    /// Create and configure a SCSS compiler.
    pub(super) fn new() -> Self {
        Self {
            options: Options::default(),
        }
    }

    /// Compile SCSS content of a [`Entry`].
    ///
    /// This function compiles the SCSS code to CSS in the `content` property.
    /// The `format` property is set to `css`.
    pub(super) fn compile_entry(&self, entry: Entry) -> Result<Entry, Error> {
        if let Some(content) = entry.content.as_ref() {
            let content = self.compile(content).map_err(|error| Error::CompileScss {
                input_path: entry.input_path_buf(),
                source: error,
            })?;

            // Change extension to `css`
            let url = entry
                .url
                .rsplit_once('.')
                .filter(|(_, extension)| ["css", "scss"].contains(extension))
                .map(|(stem, _)| [stem, "css"].join("."))
                .unwrap_or([entry.url.as_str(), "css"].join("."));

            return Ok(Entry {
                content: Some(content),
                format: "css".to_owned(),
                url,
                ..entry
            });
        }

        Ok(entry)
    }

    /// Compile a string from SCSS to CSS.
    fn compile<S>(&self, input: S) -> anyhow::Result<String>
    where
        S: AsRef<str>,
    {
        let input = input.as_ref();

        let output = grass::from_string(input, &self.options)?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                ".outer {\n",          //
                "  .inner {\n",        //
                "    color: black;\n", //
                "  }",                 //
                "}"
            ),
            ".outer .inner {\n  color: black;\n}\n",
        )];

        let compiler = super::Compiler::new();

        for (input, expected) in CASES {
            let result = compiler.compile(input).unwrap();
            assert_eq!(
                result,
                expected.to_owned(),
                "\ncompile({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
