//! Load configuration from Rhai scripts.

use std::{path::Path, sync::Arc};

use super::PartialConfig;
use crate::util::from_rhai::FromRhai;

/// Load configuration from a Rhai file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a Rhai string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Initialize the rhai engine
    let engine = Arc::new(rhai::Engine::new());

    // Compile the script
    let ast = Arc::new(engine.compile(content)?);

    // Execute the script
    let result: rhai::Dynamic = engine.eval_ast(&ast)?;

    let result = PartialConfig::from_rhai(&result, engine, ast)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
        #{
            input_dir: "foo",
            output_dir: "bar",
            base_url: "/baz",
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

        assert_eq!(config.input_dir.unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap(), "_layouts");
        let layout_filters = config.layout_filters.unwrap();
        assert_eq!(layout_filters.len(), 1);
        assert!(layout_filters.contains_key("upper"));
        let layout_functions = config.layout_functions.unwrap();
        assert_eq!(layout_functions.len(), 1);
        assert!(layout_functions.contains_key("min"));
        let layout_testers = config.layout_testers.unwrap();
        assert_eq!(layout_testers.len(), 1);
        assert!(layout_testers.contains_key("odd"));
        assert_eq!(config.syntax_highlight_css_prefix.unwrap(), "highlight-");
        assert_eq!(
            config.syntax_highlight_stylesheets.as_ref().unwrap().len(),
            1
        );
        let stylesheet = config
            .syntax_highlight_stylesheets
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap();
        assert_eq!(stylesheet.prefix.as_ref().unwrap(), "highlight-");
        assert_eq!(stylesheet.theme, "base16-ocean.dark");
        assert_eq!(stylesheet.url, "/highlight.css");
    }
}
