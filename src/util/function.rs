//! Generic function handler.

mod js;
mod lua;
mod rhai;

use std::sync::Arc;

use super::{from_js::FromJs, from_lua::FromLua, from_rhai::FromRhai};

/// Generic function handler.
#[derive(Clone)]
pub(crate) enum Function {
    Js(self::js::Function),
    Lua(self::lua::Function),
    Rhai(self::rhai::Function),
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Js(function) => function.fmt(f),
            Self::Lua(function) => function.fmt(f),
            Self::Rhai(function) => function.fmt(f),
        }
    }
}

/// Generate a `call_N(...)` method for [`Function`].
macro_rules! impl_function_call {
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
            match self {
                Self::Js(function) => function.$method_name($($arg_name),*),
                Self::Lua(function) => function.$method_name($($arg_name),*),
                Self::Rhai(function) => function.$method_name($($arg_name),*),
            }
        }
    }
}

impl Function {
    impl_function_call!(call_1(a1: A1));

    impl_function_call!(call_2(a1: A1, a2: A2));
}

impl FromJs for Function {
    fn from_js(
        value: quickjs_runtime::values::JsValueFacade,
        runtime: Arc<quickjs_runtime::facades::QuickJsRuntimeFacade>,
    ) -> anyhow::Result<Self> {
        Ok(Function::Js(self::js::Function::from_js(value, runtime)?))
    }
}

impl FromLua for Function {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> anyhow::Result<Self> {
        Ok(Function::Lua(self::lua::Function::from_lua(value, lua)?))
    }
}

impl FromRhai for Function {
    fn from_rhai(
        value: &::rhai::Dynamic,
        engine: Arc<::rhai::Engine>,
        ast: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        Ok(Function::Rhai(self::rhai::Function::from_rhai(
            value, engine, ast,
        )?))
    }
}
