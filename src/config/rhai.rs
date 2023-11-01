//! Load configuration from Rhai scripts.

use std::{collections::HashMap, path::Path, sync::Arc};

use rhai::{Array, Dynamic, Engine, Map, AST};

use super::PartialConfig;

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
    let engine = Arc::new(Engine::new());

    // Compile the script
    let ast = Arc::new(engine.compile(content)?);

    // Execute the script
    let result = engine.eval_ast::<rhai::Map>(&ast)?;

    Ok(PartialConfig {
        input_dir: result
            .get("input_dir")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?,
        output_dir: result
            .get("output_dir")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?,
        base_url: result
            .get("base_url")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?,
        data_dir: result
            .get("data_dir")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?,
        layout_dir: result
            .get("layout_dir")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?,
        layout_filters: result
            .get("layout_filters")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        layout_functions: result
            .get("layout_functions")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        layout_testers: result
            .get("layout_testers")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        syntax_highlight_code_attributes: result
            .get("syntax_highlight_code_attributes")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        syntax_highlight_pre_attributes: result
            .get("syntax_highlight_pre_attributes")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        syntax_highlight_css_prefix: result
            .get("syntax_highlight_css_prefix")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        syntax_highlight_formatter: result
            .get("syntax_highlight_formatter")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
        syntax_highlight_stylesheets: result
            .get("syntax_highlight_stylesheets")
            .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
            .transpose()?
            .unwrap_or_default(),
    })
}

/// Trait for types convertible from [`Dynamic`].
pub(super) trait FromRhai: Sized {
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self>;
}

impl FromRhai for String {
    fn from_rhai(value: &Dynamic, _: Arc<Engine>, _: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .into_string()
            .map_err(|error| anyhow::anyhow!(error))
    }
}

impl<T> FromRhai for Option<T>
where
    T: FromRhai,
{
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self> {
        if value.is_unit() {
            Ok(None)
        } else {
            FromRhai::from_rhai(&value, Arc::clone(&engine), Arc::clone(&ast))
        }
    }
}

impl<T> FromRhai for Vec<T>
where
    T: FromRhai,
{
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .try_cast::<Array>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "error converting {} to {}",
                    value.type_name(),
                    stringify!(Self)
                )
            })?
            .into_iter()
            .map(|value| FromRhai::from_rhai(&value, Arc::clone(&engine), Arc::clone(&ast)))
            .collect()
    }
}

impl<K, V, S> FromRhai for HashMap<K, V, S>
where
    K: Eq + std::hash::Hash + From<String>,
    V: FromRhai,
    S: std::hash::BuildHasher + Default,
{
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .try_cast::<Map>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "error converting {} to {}",
                    value.type_name(),
                    stringify!(Self)
                )
            })?
            .into_iter()
            .map(|(key, value)| {
                Ok((
                    key.to_string().into(),
                    FromRhai::from_rhai(&value, Arc::clone(&engine), Arc::clone(&ast))?,
                ))
            })
            .collect()
    }
}

/// Implement [`FromRhai`] for layout engine filters/functions/testers.
macro_rules! impl_from_rhai_for_layout_fn {
    (
        $($struct_name:ident)::*: ($($arg_name:ident: $arg_type:ty),*) -> $ret_type:ty
    ) => {
        impl FromRhai for $($struct_name)::* {
            fn from_rhai(
                value: &::rhai::Dynamic,
                engine: Arc<::rhai::Engine>,
                ast: Arc<::rhai::AST>,
            ) -> anyhow::Result<Self> {
                let fn_ptr = value.to_owned().try_cast::<::rhai::FnPtr>().ok_or_else(|| {
                    ::anyhow::anyhow!(
                        "expected {}, received {}",
                        stringify!(::rhai::FnPtr),
                        value.type_name()
                    )
                })?;

                Ok($($struct_name)::*(Box::new(
                    move |$($arg_name: $arg_type),*| -> $ret_type {
                        $(
                            let $arg_name = ::rhai::serde::to_dynamic($arg_name)
                                .map_err(|error| ::tera::Error::msg(error.to_string()))?
                                .to_owned();
                        )*

                        let result = fn_ptr
                            .call::<::rhai::Dynamic>(&engine, &ast, ($($arg_name),*,))
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        let result = ::rhai::serde::from_dynamic(&result)
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        Ok(result)
                    },
                )))
            }
        }
    }
}

impl_from_rhai_for_layout_fn!(
    super::LayoutFilterFn:
        (value: &tera::Value, args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

impl_from_rhai_for_layout_fn!(
    super::LayoutFunctionFn: (args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

impl_from_rhai_for_layout_fn!(
    super::LayoutTesterFn: (value: Option<&tera::Value>, args: &[tera::Value]) -> tera::Result<bool>
);

impl FromRhai for super::SyntaxHighlightFormatterFn {
    fn from_rhai(
        value: &::rhai::Dynamic,
        engine: Arc<::rhai::Engine>,
        ast: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        use super::SyntaxHighlightFormatterFn;

        let fn_ptr = value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
            anyhow::anyhow!(
                "expected {}, received {}",
                stringify!(rhai::FnPtr),
                value.type_name()
            )
        })?;

        Ok(SyntaxHighlightFormatterFn(Arc::new(
            move |content: &String,
                  attributes: &HashMap<String, String>|
                  -> anyhow::Result<Option<String>> {
                let content = rhai::serde::to_dynamic(content)?.to_owned();

                let attributes = rhai::serde::to_dynamic(attributes)?.to_owned();

                let result = fn_ptr.call::<rhai::Dynamic>(&engine, &ast, (content, attributes))?;

                let result = rhai::serde::from_dynamic(&result)?;

                Ok(result)
            },
        )))
    }
}

impl FromRhai for super::SyntaxHighlightStylesheet {
    fn from_rhai(
        value: &::rhai::Dynamic,
        _: Arc<::rhai::Engine>,
        _: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        ::rhai::serde::from_dynamic(value).map_err(|error| anyhow::anyhow!(error))
    }
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
    fn load_config_str_empty() {
        const CONTENT: &str = "#{}";

        let config = super::load_config_str(CONTENT).unwrap();

        assert!(config.input_dir.is_none());
        assert!(config.output_dir.is_none());
        assert!(config.base_url.is_none());
        assert!(config.data_dir.is_none());
        assert!(config.layout_dir.is_none());
        assert!(config.layout_filters.is_empty(),);
        assert!(config.layout_functions.is_empty());
        assert!(config.layout_testers.is_empty());
        assert!(config.syntax_highlight_css_prefix.is_empty());
        assert!(config.syntax_highlight_stylesheets.is_empty());
    }
}
