//! Load configuration from YAML files.

use std::path::Path;

use super::{Error, PartialConfig};

/// Load configuration from a YAML file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let content = std::fs::read_to_string(path).map_err(|error| Error::LoadConfig {
        config_path: Some(path.to_owned()),
        source: error.into(),
    })?;

    load_config_str(content)
}

/// Load configuration from a YAML string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
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
            base_url: /baz
            data_dir: _data
            layout_dir: _layouts
        "#;

        let config = super::load_config_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
    }
}
