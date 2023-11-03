//! Load configuration from Rhai scripts.

use std::{collections::HashMap, path::Path, sync::Arc};

use rhai::{Dynamic, Engine, AST};

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
    let result: Dynamic = engine.eval_ast(&ast)?;

    let result = FromRhai::from_rhai(&result, engine, ast)?;

    Ok(result)
}

#[derive(Clone)]
pub(crate) struct Function {
    engine: Arc<rhai::Engine>,
    ast: Arc<rhai::AST>,
    fn_ptr: Arc<rhai::FnPtr>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rhai::function::{:?}", self.fn_ptr)
    }
}

/// Generate a `call_N()` method for [`Function`].
macro_rules! impl_rhai_function_call {
    (
        $(#[$($attrs:tt)*])*
        $method_name:ident($($arg_name:ident: $arg_type:tt),*)
    ) => {
        pub(crate) fn $method_name<$($arg_type,)* R>(&self, $($arg_name: &$arg_type),*)
            -> anyhow::Result<R>
        where
            $(
                $arg_type: serde::Serialize + ?Sized,
            )*
            R: serde::de::DeserializeOwned,
        {
            $(
                let $arg_name = rhai::serde::to_dynamic($arg_name)?.to_owned();
            )*

            let result = self
                .fn_ptr
                .call::<rhai::Dynamic>(&self.engine, &self.ast, ($($arg_name,)*))?;

            let result = rhai::serde::from_dynamic(&result)?;

            Ok(result)
        }
    }
}

impl Function {
    impl_rhai_function_call!(call_1(a1: A1));

    impl_rhai_function_call!(call_2(a1: A1, a2: A2));
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
            FromRhai::from_rhai(&value, engine, ast)
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
            .try_cast::<rhai::Array>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "error converting {} to {}",
                    value.type_name(),
                    stringify!(Vec<T>)
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
            .try_cast::<rhai::Map>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "error converting {} to {}",
                    value.type_name(),
                    stringify!(HashMap<K, V, S>)
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

impl FromRhai for Function {
    fn from_rhai(
        value: &rhai::Dynamic,
        engine: Arc<rhai::Engine>,
        ast: Arc<rhai::AST>,
    ) -> anyhow::Result<Self> {
        let fn_ptr = Arc::new(value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
            anyhow::anyhow!(
                "expected {}, received {}",
                stringify!(rhai::FnPtr),
                value.type_name()
            )
        })?);

        Ok(Function {
            engine,
            ast,
            fn_ptr,
        })
    }
}

impl FromRhai for super::PartialConfig {
    fn from_rhai(
        value: &::rhai::Dynamic,
        engine: Arc<::rhai::Engine>,
        ast: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        let map = value.to_owned().try_cast::<rhai::Map>().ok_or_else(|| {
            anyhow::anyhow!(
                "error converting {} to {}",
                value.type_name(),
                stringify!(rhai::Map)
            )
        })?;

        macro_rules! partial_config {
            ($($key:ident,)*) => {
                PartialConfig {
                    $($key: map
                        .get(stringify!($key))
                        .map(|v| FromRhai::from_rhai(v, Arc::clone(&engine), Arc::clone(&ast)))
                        .transpose()?,)*
                }
            }
        }

        Ok(partial_config!(
            input_dir,
            output_dir,
            base_url,
            data_dir,
            layout_dir,
            layout_filters,
            layout_functions,
            layout_testers,
            syntax_highlight_code_attributes,
            syntax_highlight_pre_attributes,
            syntax_highlight_css_prefix,
            syntax_highlight_formatter,
            syntax_highlight_stylesheets,
        ))
    }
}

impl FromRhai for super::PartialSyntaxHighlightStylesheet {
    fn from_rhai(
        value: &::rhai::Dynamic,
        _: Arc<::rhai::Engine>,
        _: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        rhai::serde::from_dynamic(value).map_err(|error| anyhow::anyhow!(error))
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
