//! Load configuration from Lua scripts.

#[cfg(test)]
mod tests {
    #[test]
    fn load_str_derive() {
        use vitrine_derive::FromLua;

        #[derive(FromLua)]
        struct Config {
            #[vitrine(skip)]
            skip_value: String,

            #[vitrine(default)]
            default_value: String,

            #[vitrine(default = "default_value")]
            default_value_function: String,
        }

        fn default_value() -> String {
            "baz".to_owned()
        }

        const CONTENT: &str = r#"
        return {
            skip_value = "foo",
            default_value = "bar",
            default_value_function = "foobar",
        }
        "#;

        let config: Config = crate::util::data::lua::read_str(CONTENT).unwrap();

        assert_eq!(config.skip_value, "");
        assert_eq!(config.default_value, "bar");
        assert_eq!(config.default_value_function, "foobar");
    }

    #[test]
    fn load_str_derive_empty() {
        use vitrine_derive::FromLua;

        #[derive(FromLua)]
        struct Config {
            #[vitrine(skip)]
            skip_value: String,

            #[vitrine(default)]
            default_value: String,

            #[vitrine(default = "default_value")]
            default_value_function: String,
        }

        fn default_value() -> String {
            "baz".to_owned()
        }

        let config: Config = crate::util::data::lua::read_str("return {}").unwrap();

        assert_eq!(config.skip_value, "");
        assert_eq!(config.default_value, "");
        assert_eq!(config.default_value_function, "baz");
    }

    #[test]
    fn load_config_str() {
        use super::super::super::Config;

        const CONTENT: &str = r#"
        return {
            input_dir = "foo",
            output_dir = "bar",
            base_url = "/blog",
            data_dir = "_data",
            layout_dir = "_layouts",
            layout = {
                filters = {
                    upper = function(value, args) return string.upper(value) end,
                },
                functions = {
                    min = function(args) return math.min(unpack(args.values)) end,
                },
                testers = {
                    odd = function(value, args) return value % 2 == 1 end,
                },
            },
            syntax_highlight = {
                css_prefix = "highlight-",
                stylesheets = {
                    {
                        prefix = "highlight-",
                        theme = "base16-ocean.dark",
                        url = "/highlight.css",
                    },
                },
            },
        }
        "#;

        let config: Config = crate::util::data::lua::read_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url, "/blog");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(config.layout.filters.len(), 1);
        assert!(config.layout.filters.contains_key("upper"));
        assert_eq!(
            config
                .layout
                .filters
                .get("upper")
                .unwrap()
                .call_2::<_, _, tera::Value>(
                    &tera::Value::from("Hello"),
                    &tera::Value::from(tera::Map::new())
                )
                .unwrap()
                .as_str()
                .unwrap(),
            "HELLO"
        );
        assert_eq!(config.layout.functions.len(), 1);
        assert!(config.layout.functions.contains_key("min"));
        assert_eq!(
            config
                .layout
                .functions
                .get("min")
                .unwrap()
                .call_1::<_, tera::Value>(&tera::Value::from(tera::Map::from_iter([(
                    String::from("values"),
                    tera::Value::from(Vec::from([12, 6, 24]))
                )])))
                .unwrap()
                .as_i64()
                .unwrap(),
            6
        );
        assert_eq!(config.layout.testers.len(), 1);
        assert!(config.layout.testers.contains_key("odd"));
        assert_eq!(
            config
                .layout
                .testers
                .get("odd")
                .unwrap()
                .call_2::<_, _, bool>(&tera::Value::from(1), &tera::Value::from(tera::Map::new()))
                .unwrap(),
            true
        );
        assert_eq!(config.syntax_highlight.css_prefix, "highlight-");
        assert_eq!(config.syntax_highlight.stylesheets.len(), 1);
        let stylesheet = config.syntax_highlight.stylesheets.get(0).unwrap();
        assert_eq!(stylesheet.prefix, "highlight-");
        assert_eq!(stylesheet.theme, "base16-ocean.dark");
        assert_eq!(stylesheet.url, "/highlight.css");
    }

    #[test]
    fn load_config_empty() {
        use super::super::super::Config;

        let config: Config = crate::util::data::lua::read_str("return {}").unwrap();

        assert_eq!(config.input_dir, super::super::default_input_dir());
        assert_eq!(config.output_dir, super::super::default_output_dir());
        assert_eq!(config.base_url, super::super::default_base_url());
        assert_eq!(config.data_dir, super::super::default_data_dir());
        assert_eq!(config.layout_dir, super::super::default_layout_dir());
    }
}
