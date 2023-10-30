//! Load configuration from TOML files.

use std::path::Path;

use super::PartialConfig;

/// Load configuration from a TOML file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a TOML string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
where
    S: AsRef<str>,
{
    Ok(toml::from_str(content.as_ref())?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
            input_dir = "foo"
            output_dir = "bar"
            base_url = "/baz"
            data_dir = "_data"
            layout_dir = "_layouts"
        "#;

        let config = super::load_config_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap(), "_layouts");
        assert!(config.layout_filters.is_empty());
        assert!(config.layout_functions.is_empty());
        assert!(config.layout_testers.is_empty());
        assert!(config.syntax_highlight_css_prefix.is_empty());
        assert!(config.syntax_highlight_stylesheets.is_empty());
    }

    #[test]
    fn load_config_str_empty() {
        const CONTENT: &str = "";

        let config = super::load_config_str(CONTENT).unwrap();

        assert!(config.input_dir.is_none());
        assert!(config.output_dir.is_none());
        assert!(config.base_url.is_none());
        assert!(config.data_dir.is_none());
        assert!(config.layout_dir.is_none());
        assert!(config.layout_filters.is_empty());
        assert!(config.layout_functions.is_empty());
        assert!(config.layout_testers.is_empty());
        assert!(config.syntax_highlight_css_prefix.is_empty());
        assert!(config.syntax_highlight_stylesheets.is_empty());
    }
}
