//! Generate syntax highlight themes.
//!
//! This module uses [`syntect`] under the hood.

use once_cell::sync::Lazy;
use syntect::{
    highlighting::{FontStyle, ThemeSet},
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum SyntaxHighlightError {
    /// Format error.
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
    /// Syntect error.
    #[error(transparent)]
    Syntect(#[from] syntect::Error),
    /// Theme not found error.
    #[error("theme not found")]
    ThemeNotFound,
    /// Context providing a theme name.
    #[error("theme: `{theme}`")]
    WithTheme {
        /// Source error.
        source: Box<Self>,
        /// Theme name.
        theme: String,
    },
    /// Context providing a list of available themes.
    #[error("available themes: {}", format_with_theme_list_error())]
    WithThemeList {
        /// Sourc error.
        source: Box<Self>,
    },
}

/// Set of default themes.
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Set of default syntaxes.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Syntax highlighting theme.
pub struct Theme {
    /// Syntect theme.
    theme: syntect::highlighting::Theme,

    /// Prefix for CSS class names.
    prefix: String,
}

/// Highlight a code with specified language.
///
/// SAFETY: `prefix` must outlive the function call.
pub fn highlight(
    input: impl AsRef<str>,
    language: Option<impl AsRef<str>>,
    prefix: Option<impl AsRef<str>>,
) -> Result<String, SyntaxHighlightError> {
    let input = input.as_ref();

    let syntax = language
        .and_then(|language| SYNTAX_SET.find_syntax_by_token(language.as_ref()))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    // SAFETY: syntect requires 'static lifetime
    let style = match prefix {
        None => ClassStyle::Spaced,
        Some(prefix) => ClassStyle::SpacedPrefixed {
            prefix: unsafe { std::mem::transmute::<&str, &'static str>(prefix.as_ref()) },
        },
    };

    let mut html_generator = ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAX_SET, style);

    for line in LinesWithEndings::from(input) {
        html_generator.parse_html_for_line_which_includes_newline(line)?;
    }

    Ok(html_generator.finalize())
}

impl Theme {
    /// Load a builtin theme by name.
    pub fn from_name(name: impl AsRef<str>) -> Result<Self, SyntaxHighlightError> {
        let name = name.as_ref();

        let theme = THEME_SET
            .themes
            .get(name)
            .cloned()
            .ok_or_else(|| SyntaxHighlightError::ThemeNotFound)?;

        Ok(Self {
            theme,
            prefix: String::new(),
        })
    }

    /// Set the prefix for CSS class names.
    pub fn with_prefix(self, prefix: impl AsRef<str>) -> Self {
        let prefix = prefix.as_ref();

        Self {
            prefix: prefix.to_string(),
            ..self
        }
    }

    /// Generate a CSS stylesheet for this theme.
    pub fn to_css(&self) -> Result<String, SyntaxHighlightError> {
        let mut output = String::new();
        self.write_css(&mut output)?;
        Ok(output)
    }

    /// Generate a CSS stylesheet to given writer.
    ///
    /// This method is inspired by
    /// [`syntect::html::css_for_theme_with_class_style`] and modified to fix
    /// syntect's issue [#308](<https://github.com/trishume/syntect/issues/308>).
    fn write_css(&self, mut writer: impl std::fmt::Write) -> Result<(), SyntaxHighlightError> {
        // Preamble
        writer.write_str("/*\n")?;
        if let Some(name) = &self.theme.name {
            writer.write_str(&format!(" * Theme: {}\n", name))?;
        }
        if let Some(author) = &self.theme.author {
            writer.write_str(&format!(" * Author: {}\n", author))?;
        }
        writer.write_str(" */\n\n")?;

        // Container styles
        writer.write_char('.')?;
        let class_name = escape_css_identifier(format!("{}code", self.prefix));
        writer.write_str(&class_name)?;
        writer.write_str(" {\n")?;
        if let Some(foreground) = self.theme.settings.foreground {
            writer.write_str(&format!(
                "  color: #{:02x}{:02x}{:02x};\n",
                foreground.r, foreground.g, foreground.b
            ))?;
        }
        if let Some(background) = self.theme.settings.background {
            writer.write_str(&format!(
                "  background-color: #{:02x}{:02x}{:02x};\n",
                background.r, background.g, background.b
            ))?;
        }
        writer.write_str("}\n\n")?;

        for theme_item in &self.theme.scopes {
            // Multiple selectors
            let scope_selectors = &theme_item.scope.selectors;
            for (i, scope_selector) in scope_selectors.iter().enumerate() {
                // One selector
                let scopes = scope_selector.extract_scopes();
                for (j, scope) in scopes.iter().enumerate() {
                    let scope_repo = syntect::parsing::SCOPE_REPO.lock().unwrap();
                    for k in 0..(scope.len()) {
                        let atom = scope.atom_at(k as usize);
                        let atom_str = scope_repo.atom_str(atom);
                        writer.write_char('.')?;
                        let class_name =
                            escape_css_identifier(format!("{}{}", self.prefix, atom_str));
                        writer.write_str(&class_name)?;
                    }
                    if j + 1 < scopes.len() {
                        writer.write_char(' ')?;
                    }
                }
                if i + 1 < scope_selectors.len() {
                    writer.write_str(",\n")?;
                }
            }
            writer.write_str(" {\n")?;

            // Rules
            if let Some(foreground) = theme_item.style.foreground {
                writer.write_str(&format!(
                    "  color: #{:02x}{:02x}{:02x};\n",
                    foreground.r, foreground.g, foreground.b
                ))?;
            }
            if let Some(background) = theme_item.style.background {
                writer.write_str(&format!(
                    "  background-color: #{:02x}{:02x}{:02x};\n",
                    background.r, background.g, background.b
                ))?;
            }
            if let Some(font_style) = theme_item.style.font_style {
                match font_style {
                    FontStyle::BOLD => {
                        writer.write_str("  font-weight: bold;\n")?;
                    },
                    FontStyle::ITALIC => {
                        writer.write_str("  font-style: italic;\n")?;
                    },
                    FontStyle::UNDERLINE => {
                        writer.write_str("  text-decoration: underline;\n")?;
                    },
                    _ => {},
                };
            }
            writer.write_str("}\n\n")?;
        }

        Ok(())
    }
}

/// Escape special characters in a CSS identifier.
///
/// See <https://www.w3.org/TR/CSS21/syndata.html#characters>.
fn escape_css_identifier(input: impl AsRef<str>) -> String {
    let input = input.as_ref();

    input
        .char_indices()
        .fold(String::with_capacity(input.len()), |mut output, (i, c)| {
            if !c.is_ascii_alphabetic() && c != '-' && c != '_' && (!c.is_ascii_digit() || i == 0) {
                output.push('\\');
            }
            output.push(c);
            output
        })
}

/// Return the list of themes formatted as a string.
fn format_with_theme_list_error() -> String {
    ThemeSet::load_defaults()
        .themes
        .keys()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Pipeline task.
pub mod task {
    use super::{SyntaxHighlightError, Theme};
    use crate::{
        build::Style,
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    /// Task to generate syntax highlight style sheets.
    #[derive(Debug)]
    pub struct SyntaxHighlightTask<'config> {
        config: &'config Config,
    }

    impl<'config> SyntaxHighlightTask<'config> {
        /// Create a pipeline task to generate syntax highlight style sheets.
        pub fn new(config: &'config Config) -> SyntaxHighlightTask<'config> {
            Self { config }
        }
    }

    impl Task<(Style,), (Style,), SyntaxHighlightError> for SyntaxHighlightTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Style>,),
            (tx,): (Sender<Style>,),
        ) -> Result<(), SyntaxHighlightError> {
            // Forward existing styles
            for style in rx {
                tx.send(style).unwrap();
            }

            // Create styles for syntax highlight
            for stylesheet in &self.config.syntax_highlight.stylesheets {
                let theme = Theme::from_name(&stylesheet.theme)
                    .map_err(|source| SyntaxHighlightError::WithTheme {
                        source: Box::new(source),
                        theme: stylesheet.theme.clone(),
                    })
                    .map_err(|source| SyntaxHighlightError::WithThemeList {
                        source: Box::new(source),
                    })?
                    .with_prefix(&stylesheet.prefix);

                let content = theme
                    .to_css()
                    .map_err(|source| SyntaxHighlightError::WithTheme {
                        source: Box::new(source),
                        theme: stylesheet.theme.clone(),
                    })?;

                let url = stylesheet.url.clone().into();

                tx.send(Style {
                    input_path: Default::default(),
                    url,
                    content,
                })
                .unwrap();
            }

            Ok(())
        }
    }
}
