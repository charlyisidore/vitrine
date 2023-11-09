//! Load configuration from Rhai scripts.

use std::{path::Path, sync::Arc};

use super::Config;
use crate::util::from_rhai::FromRhai;

/// Load configuration from a Rhai file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<Config>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a Rhai string.
fn load_config_str<S>(content: S) -> anyhow::Result<Config>
where
    S: AsRef<str>,
{
    load_str(content)
}

/// Load a structure from a Rhai string.
fn load_str<T, S>(content: S) -> anyhow::Result<T>
where
    T: FromRhai,
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Initialize the rhai engine
    let engine = Arc::new(rhai::Engine::new());

    // Compile the script
    let ast = Arc::new(engine.compile(content)?);

    // Execute the script
    let result: rhai::Dynamic = engine.eval_ast(&ast)?;

    let result = T::from_rhai(&result, engine, ast)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_str_derive() {
        use vitrine_derive::FromRhai;

        #[derive(FromRhai)]
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
        #{
            skip_value: "foo",
            default_value: "bar",
            default_value_function: "foobar",
        }
        "#;

        let config: Config = super::load_str(CONTENT).unwrap();

        assert_eq!(config.skip_value, "");
        assert_eq!(config.default_value, "bar");
        assert_eq!(config.default_value_function, "foobar");
    }

    #[test]
    fn load_str_derive_empty() {
        use vitrine_derive::FromRhai;

        #[derive(FromRhai)]
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

        let config: Config = super::load_str("#{}").unwrap();

        assert_eq!(config.skip_value, "");
        assert_eq!(config.default_value, "");
        assert_eq!(config.default_value_function, "baz");
    }

    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
        #{
            input_dir: "foo",
            output_dir: "bar",
            base_url: "/blog",
            data_dir: "_data",
            layout_dir: "_layouts",
            layout_filters: #{
                upper: |value, args| value.to_upper(),
            },
            layout_functions: #{
                min: |args| args.values?.reduce(|a, b| min(a, b), 0xffffffff),
            },
            layout_testers: #{
                odd: |value, args| value % 2 == 1,
            },
            syntax_highlight_css_prefix: "highlight-",
            syntax_highlight_stylesheets: [
                #{
                    prefix: "highlight-",
                    theme: "base16-ocean.dark",
                    url: "/highlight.css",
                },
            ],
        }
        "#;

        let config = super::load_config_str(CONTENT).unwrap();

        assert_eq!(config.input_dir.to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url, "/blog");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(config.layout_filters.len(), 1);
        assert!(config.layout_filters.contains_key("upper"));
        assert_eq!(config.layout_functions.len(), 1);
        assert!(config.layout_functions.contains_key("min"));
        assert_eq!(config.layout_testers.len(), 1);
        assert!(config.layout_testers.contains_key("odd"));
        assert_eq!(config.syntax_highlight_css_prefix, "highlight-");
        assert_eq!(config.syntax_highlight_stylesheets.len(), 1);
        let stylesheet = config.syntax_highlight_stylesheets.get(0).unwrap();
        assert_eq!(stylesheet.prefix, "highlight-");
        assert_eq!(stylesheet.theme, "base16-ocean.dark");
        assert_eq!(stylesheet.url, "/highlight.css");
    }

    #[test]
    fn load_config_empty() {
        let config = super::load_config_str("#{}").unwrap();

        assert_eq!(config.input_dir, super::super::default_input_dir());
        assert_eq!(config.output_dir, super::super::default_output_dir());
        assert_eq!(config.base_url, super::super::default_base_url());
        assert_eq!(config.data_dir, super::super::default_data_dir());
        assert_eq!(config.layout_dir, super::super::default_layout_dir());
    }
}
