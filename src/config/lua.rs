//! Load configuration from Lua scripts.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use mlua::{FromLua, Lua};

use super::PartialConfig;

/// Load configuration from a Lua file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a Lua string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Call `unsafe_new()` to allow loading C modules
    let lua = unsafe { Lua::unsafe_new() };

    // `Lua` is not `Sync`, so we wrap it in `Arc<Mutex>`
    let lua_mutex = Arc::new(Mutex::new(lua));
    let lua = lua_mutex.lock().unwrap();

    // Save the mutex in Lua's context, we can retrieve it with `lua.app_data_ref()`
    lua.set_app_data(Arc::clone(&lua_mutex));

    let result: mlua::Table = lua.load(content).eval()?;

    Ok(PartialConfig {
        input_dir: result.get("input_dir")?,
        output_dir: result.get("output_dir")?,
        base_url: result.get("base_url")?,
        data_dir: result.get("data_dir")?,
        layout_dir: result.get("layout_dir")?,
        layout_filters: result
            .get::<_, Option<_>>("layout_filters")?
            .unwrap_or_default(),
        layout_functions: result
            .get::<_, Option<_>>("layout_functions")?
            .unwrap_or_default(),
        layout_testers: result
            .get::<_, Option<_>>("layout_testers")?
            .unwrap_or_default(),
        syntax_highlight_code_attributes: result
            .get::<_, Option<_>>("syntax_highlight_code_attributes")?
            .unwrap_or_default(),
        syntax_highlight_pre_attributes: result
            .get::<_, Option<_>>("syntax_highlight_pre_attributes")?
            .unwrap_or_default(),
        syntax_highlight_css_prefix: result
            .get::<_, Option<_>>("syntax_highlight_css_prefix")?
            .unwrap_or_default(),
        syntax_highlight_formatter: result
            .get::<_, Option<_>>("syntax_highlight_formatter")?
            .unwrap_or_default(),
        syntax_highlight_stylesheets: result
            .get::<_, Option<_>>("syntax_highlight_stylesheets")?
            .unwrap_or_default(),
    })
}

/// Implement [`FromLua`] for layout engine filters/functions/testers.
macro_rules! impl_from_lua_for_layout_fn {
    (
        $($struct_name:ident)::*: ($($arg_name:ident: $arg_type:ty),*) -> $ret_type:ty
    ) => {
        impl<'lua> ::mlua::FromLua<'lua> for $($struct_name)::* {
            fn from_lua(
                value: ::mlua::Value<'lua>, lua: &'lua ::mlua::Lua
            ) -> ::mlua::Result<Self> {
                use ::std::sync::{Arc, Mutex};
                use ::mlua::{Lua, LuaSerdeExt};

                let function = value
                    .as_function()
                    .ok_or_else(|| ::mlua::Error::external(format!(
                        "expected {}, received {}",
                        stringify!(::mlua::Function),
                        value.type_name()
                    )))?
                    .to_owned();

                let function_key = lua.create_registry_value(function)?;

                let lua_mutex = lua
                    .app_data_ref::<Arc<Mutex<Lua>>>()
                    .ok_or_else(|| ::mlua::Error::external("missing lua app data"))?
                    .to_owned();

                Ok($($struct_name)::*(Box::new(
                    move |$($arg_name: $arg_type),*| -> $ret_type {
                        let lua = lua_mutex
                            .lock()
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        $(
                            let $arg_name = lua.to_value(&$arg_name)
                                .map_err(|error| ::tera::Error::msg(error.to_string()))?;
                        )*

                        let function: ::mlua::Function = lua.registry_value(&function_key)
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        let result = function.call::<_, ::mlua::Value>(($($arg_name),*))
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        let result = lua.from_value(result)
                            .map_err(|error| ::tera::Error::msg(error.to_string()))?;

                        Ok(result)
                    },
                )))
            }
        }
    }
}

impl_from_lua_for_layout_fn!(
    super::LayoutFilterFn:
        (value: &tera::Value, args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

impl_from_lua_for_layout_fn!(
    super::LayoutFunctionFn: (args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

impl_from_lua_for_layout_fn!(
    super::LayoutTesterFn: (value: Option<&tera::Value>, args: &[tera::Value]) -> tera::Result<bool>
);

impl<'lua> FromLua<'lua> for super::SyntaxHighlightFormatterFn {
    fn from_lua(value: ::mlua::Value<'lua>, lua: &'lua ::mlua::Lua) -> ::mlua::Result<Self> {
        use ::mlua::LuaSerdeExt;

        let function = value
            .as_function()
            .ok_or_else(|| {
                ::mlua::Error::external(format!(
                    "expected {}, received {}",
                    stringify!(::mlua::Function),
                    value.type_name()
                ))
            })?
            .to_owned();

        let function_key = lua.create_registry_value(function)?;

        let lua_mutex = lua
            .app_data_ref::<Arc<Mutex<Lua>>>()
            .ok_or_else(|| ::mlua::Error::external("missing lua app data"))?
            .to_owned();

        Ok(super::SyntaxHighlightFormatterFn(Arc::new(
            move |content: &String,
                  attributes: &HashMap<String, String>|
                  -> ::anyhow::Result<Option<String>> {
                let lua = lua_mutex
                    .lock()
                    .map_err(|error| ::anyhow::anyhow!(error.to_string()))?;

                let content = lua.to_value(&content)?;

                let attributes = lua.to_value(&attributes)?;

                let function: ::mlua::Function = lua.registry_value(&function_key)?;

                let result = function.call::<_, ::mlua::Value>((content, attributes))?;

                let result = lua.from_value(result)?;

                Ok(result)
            },
        )))
    }
}

impl<'lua> FromLua<'lua> for super::SyntaxHighlightStylesheet {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua Lua) -> mlua::Result<Self> {
        use mlua::LuaSerdeExt;
        lua.from_value(value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_str() {
        const CONTENT: &str = r#"
        return {
            input_dir = "foo",
            output_dir = "bar",
            base_url = "/baz",
            data_dir = "_data",
            layout_dir = "_layouts",
            layout_filters = {
                upper = function(value, args) return string.upper(value) end,
            },
            layout_functions = {
                min = function(args) return math.min(table.unpack(args.values)) end,
            },
            layout_testers = {
                odd = function(value, args) return value % 2 == 1 end,
            },
            syntax_highlight_css_prefix = "highlight-",
            syntax_highlight_stylesheets = {
                {
                    prefix = "highlight-",
                    theme = "base16-ocean.dark",
                    url = "/highlight.css",
                },
            },
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
        const CONTENT: &str = "return {}";

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
