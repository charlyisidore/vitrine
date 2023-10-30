//! Syntax highlighting.
//!
//! This module uses [`syntect`] under the hood.

use syntect::html::{css_for_theme_with_class_style, ClassStyle};

use super::{Config, Entry, Error};

/// Create CSS stylesheet entries for syntax highlighting.
///
/// This function reads `syntax_highlight_stylesheets` in the configuration, and
/// generates CSS files corresponding to specified themes. The CSS files will be
/// written at specified URLs.
pub(super) fn create_stylesheet_entries<'config>(
    config: &'config Config,
) -> impl Iterator<Item = Result<Entry, Error>> + 'config {
    config.syntax_highlight_stylesheets.iter().map(|entry| {
        create_css(&entry.theme, &entry.prefix)
            .map_err(|error| Error::CreateSyntaxHighlightStylesheet {
                source: anyhow::anyhow!(error),
            })
            .and_then(move |content| {
                content.ok_or_else(|| Error::CreateSyntaxHighlightStylesheet {
                    source: anyhow::anyhow!("Syntax highlight theme {:?} not found", entry.theme)
                        .context(format!("Available themes: {:?}", get_themes())),
                })
            })
            .map(|content| {
                // The produced stylesheet might contain invalid characters
                // See <https://github.com/trishume/syntect/issues/308>
                escape_css(content)
            })
            .map(|content| Entry {
                content: Some(content),
                url: entry.url.to_owned(),
                format: "css".to_owned(),
                ..Default::default()
            })
    })
}

/// Create a CSS string for syntax highlighting.
fn create_css<ST, SP>(theme_key: ST, prefix: SP) -> Result<Option<String>, syntect::Error>
where
    ST: AsRef<str>,
    SP: AsRef<str>,
{
    let theme_key = theme_key.as_ref();

    // Since [`syntect`]` requires `'static` lifetime for `prefix` in
    // [`syntect::html::ClassStyle::SpacedPrefixed`], we cannot use a value created
    // at runtime. Therefore, we use `static_lifetime()` as a workaround.
    let prefix = unsafe { crate::util::r#unsafe::static_lifetime(prefix.as_ref()) };

    syntect::highlighting::ThemeSet::load_defaults()
        .themes
        .get(theme_key)
        .map(|theme| css_for_theme_with_class_style(theme, ClassStyle::SpacedPrefixed { prefix }))
        .transpose()
}

/// Get the list of themes for syntax highlighting.
fn get_themes() -> Vec<String> {
    syntect::highlighting::ThemeSet::load_defaults()
        .themes
        .keys()
        .map(|v| v.to_owned())
        .collect()
}

/// Escape some invalid characters in a CSS string.
///
/// See <https://github.com/trishume/syntect/issues/308>.
fn escape_css(content: String) -> String {
    content.replace("c++", "c\\+\\+")
}
