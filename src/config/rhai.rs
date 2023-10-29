//! Load configuration from Rhai scripts.

use std::{collections::HashMap, path::Path, sync::Arc};

use super::{
    Error, LayoutFilter, LayoutFunction, LayoutTester, PartialConfig, SyntaxHighlightStylesheet,
};

/// Load configuration from a Rhai file.
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
    let config = engine
        .eval_ast::<rhai::Dynamic>(&ast)?
        .try_cast::<rhai::Map>()
        .ok_or_else(|| anyhow::anyhow!("The configuration script must return an object"))?;

    // Input directory
    let input_dir = config
        .get("input_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Output directory
    let output_dir = config
        .get("output_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Base URL
    let base_url = config
        .get("base_url")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Data directory
    let data_dir = config
        .get("data_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Layout directory
    let layout_dir = config
        .get("layout_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Filters for the layout engine
    let layout_filters = config
        .get("layout_filters")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutFilter), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_filters must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_filter =
                                move |value: &tera::Value,
                                      args: &HashMap<String, tera::Value>|
                                      -> tera::Result<tera::Value> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let value = rhai::serde::to_dynamic(value)?.to_owned();
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn.call::<rhai::Dynamic>(
                                            &engine,
                                            &ast,
                                            (value, args),
                                        )?;

                                        // Convert result from rhai to tera types
                                        let result =
                                            rhai::serde::from_dynamic::<tera::Value>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_filter)))
                        },
                    )
                    .collect()
            },
        )?;

    // Functions for the layout engine
    let layout_functions = config
        .get("layout_functions")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutFunction), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_functions must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_function =
                                move |args: &HashMap<String, tera::Value>|
                                      -> tera::Result<tera::Value> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn
                                            .call::<rhai::Dynamic>(&engine, &ast, (args,))?;

                                        // Convert result from rhai to tera types
                                        let result =
                                            rhai::serde::from_dynamic::<tera::Value>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_function)))
                        },
                    )
                    .collect()
            },
        )?;

    // Testers for the layout engine
    let layout_testers = config
        .get("layout_testers")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutTester), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_testers must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_tester =
                                move |value: Option<&tera::Value>,
                                      args: &[tera::Value]|
                                      -> tera::Result<bool> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let value = rhai::serde::to_dynamic(value)?.to_owned();
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn.call::<rhai::Dynamic>(
                                            &engine,
                                            &ast,
                                            (value, args),
                                        )?;

                                        // Convert result from rhai to tera types
                                        let result = rhai::serde::from_dynamic::<bool>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_tester)))
                        },
                    )
                    .collect()
            },
        )?;

    // Prefix for syntax highlight CSS classes
    let syntax_highlight_css_prefix = config
        .get("syntax_highlight_css_prefix")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .unwrap_or_default();

    // Syntax highlight CSS stylesheets
    let syntax_highlight_stylesheets = config
        .get("syntax_highlight_stylesheets")
        .and_then(|v| v.to_owned().try_cast::<rhai::Array>())
        .map_or_else(
            || Ok(Vec::new()),
            |array| {
                array
                    .iter()
                    .map(|v| {
                        v.to_owned()
                            .try_cast::<rhai::Map>()
                            .map(|v| {
                                Ok::<_, &str>(SyntaxHighlightStylesheet {
                                    prefix: v
                                        .get("prefix")
                                        .map(|v| v.to_owned().into_string())
                                        .transpose()?
                                        .unwrap_or_default(),
                                    theme: v
                                        .get("theme")
                                        .ok_or_else(|| {
                                            "Missing theme in syntax_highlight_stylesheets"
                                        })?
                                        .to_owned()
                                        .into_string()?,
                                    url: v
                                        .get("url")
                                        .ok_or_else(|| {
                                            "Missing url in syntax_highlight_stylesheets"
                                        })?
                                        .to_owned()
                                        .into_string()?,
                                })
                            })
                            .transpose()
                            .map_err(|error| anyhow::anyhow!(error))?
                            .ok_or_else(|| {
                                anyhow::anyhow!("Cannot parse syntax_highlight_stylesheets")
                            })
                    })
                    .collect()
            },
        )?;

    Ok(PartialConfig {
        input_dir,
        output_dir,
        base_url,
        data_dir,
        layout_dir,
        layout_filters,
        layout_functions,
        layout_testers,
        syntax_highlight_css_prefix,
        syntax_highlight_stylesheets,
        ..Default::default()
    })
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

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
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
}
