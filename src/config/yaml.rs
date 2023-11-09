//! Load configuration from YAML files.

use std::path::Path;

use super::Config;

/// Load configuration from a YAML file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<Config>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a YAML string.
fn load_config_str<S>(content: S) -> anyhow::Result<Config>
where
    S: AsRef<str>,
{
    Ok(serde_yaml::from_str(content.as_ref())?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
            input_dir: foo
            output_dir: bar
            base_url: /blog
            data_dir: _data
            layout_dir: _layouts
            syntax_highlight:
              pre_attributes:
                class: 'syntax-highlight'
        "#;

        let config = super::load_config_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url, "/blog");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(
            config.syntax_highlight.pre_attributes.get("class").unwrap(),
            "syntax-highlight"
        );
    }

    #[test]
    fn load_config_empty() {
        let config = super::load_config_str("").unwrap();

        assert_eq!(config.input_dir, super::super::default_input_dir());
        assert_eq!(config.output_dir, super::super::default_output_dir());
        assert_eq!(config.base_url, super::super::default_base_url());
        assert_eq!(config.data_dir, super::super::default_data_dir());
        assert_eq!(config.layout_dir, super::super::default_layout_dir());
    }
}
