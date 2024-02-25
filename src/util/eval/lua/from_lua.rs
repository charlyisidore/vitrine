//! Convert values from [`mlua::Value`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use mlua::Lua;

use super::{into_lua::IntoLua, LuaError};
use crate::util::function::Function;

/// Trait for types convertible from [`mlua::Value`].
///
/// This implementation does not require any litetime specifier.
pub trait FromLua
where
    Self: Sized,
{
    fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError>;
}

/// Implements [`FromLua`] for types implementing [`mlua::FromLua`].
macro_rules! impl_from_lua_from_lua {
    ($ty:ty) => {
        impl FromLua for $ty {
            fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
                Ok(mlua::FromLua::from_lua(value, lua)?)
            }
        }
    };
}

impl_from_lua_from_lua! { bool }
impl_from_lua_from_lua! { f32 }
impl_from_lua_from_lua! { f64 }
impl_from_lua_from_lua! { i8 }
impl_from_lua_from_lua! { i16 }
impl_from_lua_from_lua! { i32 }
impl_from_lua_from_lua! { i64 }
impl_from_lua_from_lua! { i128 }
impl_from_lua_from_lua! { isize }
impl_from_lua_from_lua! { u8 }
impl_from_lua_from_lua! { u16 }
impl_from_lua_from_lua! { u32 }
impl_from_lua_from_lua! { u64 }
impl_from_lua_from_lua! { u128 }
impl_from_lua_from_lua! { usize }
impl_from_lua_from_lua! { String }

impl FromLua for PathBuf {
    fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
        let type_name = value.type_name();
        Ok(lua
            .coerce_string(value)?
            .ok_or_else(|| mlua::Error::FromLuaConversionError {
                from: type_name,
                to: "PathBuf",
                message: Some("expected string or number".to_string()),
            })?
            .to_str()?
            .into())
    }
}

impl<T> FromLua for Option<T>
where
    T: FromLua,
{
    fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
        match value {
            mlua::Nil => Ok(None),
            value => Ok(Some(T::from_lua(value, lua)?)),
        }
    }
}

/// Implements [`FromLua`] for array types.
macro_rules! impl_from_lua_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromLua for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
                match value {
                    mlua::Value::Table(table) => table
                        .sequence_values()
                        .map(|v| FromLua::from_lua(v?, lua))
                        .collect(),
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($ty),
                        message: Some("expected table".to_string()),
                    }
                    .into()),
                }
            }
        }
    }
}

impl_from_lua_array! { Vec<T> where T: FromLua }
impl_from_lua_array! { BTreeSet<T> where T: FromLua + Ord }
impl_from_lua_array! { HashSet<T, S> where T: Eq + FromLua + Hash, S: Default + BuildHasher }

/// Implements [`FromLua`] for table types.
macro_rules! impl_from_lua_table {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromLua for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
                match value {
                    mlua::Value::Table(table) => table
                        .pairs()
                        .map(|pair| {
                            let (k, v) = pair?;
                            Ok((FromLua::from_lua(k, lua)?, FromLua::from_lua(v, lua)?))
                        })
                        .collect(),
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: "Function",
                        message: Some("expected table".to_string()),
                    }
                    .into()),
                }
            }
        }
    }
}

impl_from_lua_table! { BTreeMap<K, V> where K: FromLua + Ord, V: FromLua }
impl_from_lua_table! {
    HashMap<K, V, S>
    where
        K: Eq + FromLua + Hash,
        V: FromLua,
        S: Default + BuildHasher
}

/// Implements [`FromLua`] for deserializable types.
macro_rules! impl_from_lua_serde {
    ($($ty:tt)*) => {
        impl FromLua for $($ty)* {
            fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
                use mlua::LuaSerdeExt;
                Ok(lua.from_value(value)?)
            }
        }
    }
}

impl_from_lua_serde! { serde_json::Value }
impl_from_lua_serde! { toml::Value }
impl_from_lua_serde! { serde_yaml::Value }

/// Implements [`FromLua`] for [`Function`].
macro_rules! impl_from_lua_fn {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> FromLua for Function <($($ty,)*), R>
        where
            $($ty: IntoLua,)*
            R: FromLua,
        {
            fn from_lua(value: mlua::Value, lua: &Lua) -> Result<Self, LuaError> {
                let type_name = value.type_name();
                match value {
                    mlua::Value::Function(function) => {
                        let key = lua.create_registry_value(function)?;
                        let lua = lua
                            .app_data_ref::<Arc<Mutex<Lua>>>()
                            .ok_or_else(|| mlua::Error::FromLuaConversionError {
                                from: type_name,
                                to: "Function",
                                message: Some("failed to get Lua app data".to_string()),
                            })?
                            .to_owned();

                        Ok(Self::from(move |$($arg: $ty),*| {
                            let lua = lua.lock().unwrap();
                            let function: mlua::Function = lua.registry_value(&key)?;
                            let args = ($($ty::into_lua($arg, &lua)?,)*);
                            let result = function.call(args)?;
                            R::from_lua(result, &lua)
                        }))
                    },
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: type_name,
                        to: "Function",
                        message: Some("expected function".to_string()),
                    }
                    .into()),
                }
            }
        }
    }
}

impl_from_lua_fn! {}
impl_from_lua_fn! { a1: A1 }
impl_from_lua_fn! { a1: A1, a2: A2 }
impl_from_lua_fn! { a1: A1, a2: A2, a3: A3 }
impl_from_lua_fn! { a1: A1, a2: A2, a3: A3, a4: A4 }
impl_from_lua_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5 }
impl_from_lua_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6 }
