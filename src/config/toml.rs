//! Load configuration from TOML files.

#[cfg(test)]
mod tests {
    use super::super::Config;

    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
            input_dir = "foo"
            output_dir = "bar"
            base_url = "/blog"
            data_dir = "_data"
            layouts_dir = "_layouts"

            [syntax_highlight]
            pre_attributes = { class = "syntax-highlight" }
        "#;

        let config: Config = crate::util::data::toml::read_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url, "/blog");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layouts_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(
            config.syntax_highlight.pre_attributes.get("class").unwrap(),
            "syntax-highlight"
        );
    }

    #[test]
    fn load_config_empty() {
        let config: Config = crate::util::data::toml::read_str("").unwrap();

        assert_eq!(config.input_dir, super::super::default_input_dir());
        assert_eq!(config.output_dir, super::super::default_output_dir());
        assert_eq!(config.base_url, super::super::default_base_url());
        assert_eq!(config.data_dir, super::super::default_data_dir());
        assert_eq!(config.layouts_dir, super::super::default_layouts_dir());
    }
}
