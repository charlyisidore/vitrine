//! Load configuration from Lua scripts.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use mlua::Lua;

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

    // Execute the script
    let result: mlua::Value = lua.load(content).eval()?;

    let result = mlua::FromLua::from_lua(result, &lua)?;

    Ok(result)
}

macro_rules! from_lua_table {
    ($value:ident, $($struct:ident)::* { $($key:ident),* }) => {
        {
            let table = $value.as_table().ok_or_else(|| {
                mlua::Error::external(format!(
                    "expected {}, received {}",
                    stringify!(mlua::Table),
                    $value.type_name()
                ))
            })?;

            $($struct)::* {
                $($key: table.get(stringify!($key))?,)*
            }
        }
    }
}

impl<'lua> mlua::FromLua<'lua> for super::PartialConfig {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        Ok(from_lua_table!(value, PartialConfig {
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
            syntax_highlight_stylesheets
        }))
    }
}

impl<'lua> mlua::FromLua<'lua> for super::PartialSyntaxHighlightStylesheet {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        Ok(from_lua_table!(
            value,
            super::PartialSyntaxHighlightStylesheet { prefix, theme, url }
        ))
    }
}

/// Lua function handler.
#[derive(Clone)]
pub(crate) struct Function {
    /// Lua engine.
    lua: Arc<Mutex<mlua::Lua>>,

    /// Registry key for the function.
    key: Arc<mlua::RegistryKey>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lua::function::{:?}", self.key)
    }
}

/// Generate a `call_N(...)` method for [`Function`].
macro_rules! impl_lua_function_call {
    (
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
            use mlua::LuaSerdeExt;

            let lua = self
                .lua
                .lock()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;

            $(
                let $arg_name = lua.to_value(&$arg_name)?;
            )*

            let function: mlua::Function = lua.registry_value(&self.key)?;

            let result = function.call::<_, mlua::Value>(($($arg_name,)*))?;

            let result = lua.from_value(result)?;

            Ok(result)
        }
    }
}

impl Function {
    impl_lua_function_call!(call_1(a1: A1));

    impl_lua_function_call!(call_2(a1: A1, a2: A2));
}

impl<'lua> mlua::FromLua<'lua> for Function {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let function = value
            .as_function()
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "expected {}, received {}",
                    stringify!(mlua::Function),
                    value.type_name()
                ))
            })?
            .to_owned();

        let key = Arc::new(lua.create_registry_value(function)?);

        let lua = lua
            .app_data_ref::<Arc<Mutex<mlua::Lua>>>()
            .ok_or_else(|| mlua::Error::external("missing lua app data"))?
            .to_owned();

        Ok(Function { lua, key })
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
