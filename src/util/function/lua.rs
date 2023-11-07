//! Lua function handler.

use std::sync::{Arc, Mutex};

use super::super::from_lua::FromLua;

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

impl FromLua<'_> for Function {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
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
