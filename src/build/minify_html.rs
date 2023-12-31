//! Minify HTML code.
//!
//! This module uses [`minify_html`] under the hood.

use minify_html::{minify, Cfg};

use super::{Entry, Error};

/// HTML minifier.
pub(super) struct Minifier {
    /// [`minify_html`] configuration.
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
    fn minify<S>(&self, input: S) -> anyhow::Result<String>
    where
        S: AsRef<str>,
    {
        let input = input.as_ref();

        // `minify()` accepts and returns a `Vec<u8>`
        let output = minify(input.as_bytes(), &self.cfg);

        // Convert `Vec<u8>` to `String`
        let output = String::from_utf8(output)?;

        // Minify `<script>` and `<style>` content
        let output = minify_inline(output)?;

        Ok(output)
    }
}

/// Minify inline JavaScript and CSS code inside `<script>` and `<style>` tags.
fn minify_inline<S>(input: S) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let input = input.as_ref();
    let mut script_buffer = String::new();
    let mut style_buffer = String::new();

    lol_html::rewrite_str(input, lol_html::RewriteStrSettings {
        element_content_handlers: vec![
            lol_html::text!("script", |element| {
                // Minify `<script>` elements
                script_buffer.push_str(element.as_str());

                if element.last_in_text_node() {
                    let content = super::minify_js::minify(&script_buffer)
                        .map_err(|error| error.context("While minifying `<script>` element"))?;

                    element.set_str(content);
                    script_buffer.clear();
                } else {
                    element.remove();
                }

                Ok(())
            }),
            lol_html::text!("style", |element| {
                // Minify `<style>` elements
                style_buffer.push_str(element.as_str());

                if element.last_in_text_node() {
                    let content = super::minify_css::minify(&style_buffer)
                        .map_err(|error| error.context("While minifying `<style>` element"))?;

                    element.set_str(content);
                    style_buffer.clear();
                } else {
                    element.remove();
                }

                Ok(())
            }),
            lol_html::element!("*[style]", |element| {
                // Minify `style`` attributes
                let Some(content) = element.get_attribute("style") else {
                    return Ok(());
                };

                const PREFIX: &str = "_{";
                const SUFFIX: &str = "}";

                // Wrap CSS rules in a fake selector to make a valid CSS stylesheet
                let content = format!("{PREFIX}{content}{SUFFIX}");

                let content = super::minify_css::minify(content)
                    .map_err(|error| error.context("While minifying `style` attribute"))?;

                if content.starts_with(PREFIX) && content.ends_with(SUFFIX) {
                    element.set_attribute(
                        "style",
                        &content[PREFIX.len()..content.len() - SUFFIX.len()],
                    )?;
                }

                Ok(())
            }),
        ],
        ..lol_html::RewriteStrSettings::default()
    })
    .map_err(|error| error.into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                "<html>\n",                                       //
                "  <head>\n",                                     //
                "    <style>body { color: black; }</style>\n",    //
                "    <script>\n",                                 //
                "      function hello() {\n",                     //
                "        return 'Hello, world!';\n",              //
                "      }\n",                                      //
                "    </script>\n",                                //
                "  </head>\n",                                    //
                "  <body style=\"background: white;\"></body>\n", //
                "</html>\n"
            ),
            "<style>body{color:#000}</style><script>var hello=(()=>`Hello, world!`)</script><body \
             style=\"background:#fff\">",
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
