//! Convert values into [`mlua::Value`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::{Path, PathBuf},
};

use mlua::Lua;

use super::LuaError;

/// Trait for types convertible into [`mlua::Value`].
///
/// This implementation does not require any litetime specifier.
pub trait IntoLua
where
    Self: Sized,
{
    fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError>;
}

/// Implements [`IntoLua`] for types implementing [`mlua::IntoLua`].
macro_rules! impl_into_lua {
    ($ty:ty) => {
        impl IntoLua for $ty {
            fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
                Ok(mlua::IntoLua::into_lua(self, lua)?)
            }
        }
    };
}

impl_into_lua! { bool }
impl_into_lua! { f32 }
impl_into_lua! { f64 }
impl_into_lua! { i8 }
impl_into_lua! { i16 }
impl_into_lua! { i32 }
impl_into_lua! { i64 }
impl_into_lua! { i128 }
impl_into_lua! { isize }
impl_into_lua! { u8 }
impl_into_lua! { u16 }
impl_into_lua! { u32 }
impl_into_lua! { u64 }
impl_into_lua! { u128 }
impl_into_lua! { usize }
impl_into_lua! { &str }
impl_into_lua! { String }

/// Implements [`IntoLua`] for path types.
macro_rules! impl_into_lua_path {
    ($ty:ty) => {
        impl IntoLua for $ty {
            fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
                Ok(mlua::Value::String(
                    lua.create_string(self.to_str().ok_or_else(|| {
                        mlua::Error::ToLuaConversionError {
                            from: stringify!($ty),
                            to: "string",
                            message: Some("invalid unicode".to_string()),
                        }
                    })?)?,
                ))
            }
        }
    };
}

impl_into_lua_path! { &Path }
impl_into_lua_path! { PathBuf }

impl<T> IntoLua for Option<T>
where
    T: IntoLua,
{
    fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
        match self {
            Some(value) => T::into_lua(value, lua),
            None => Ok(mlua::Nil),
        }
    }
}

/// Implements [`IntoLua`] for array types.
macro_rules! impl_into_lua_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoLua for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
                Ok(mlua::Value::Table(
                    lua.create_sequence_from(
                        self.into_iter()
                            .map(|v| IntoLua::into_lua(v, lua))
                            .collect::<Result<Vec<_>, LuaError>>()?,
                    )?,
                ))
            }
        }
    }
}

impl_into_lua_array! { Vec<T> where T: IntoLua }
impl_into_lua_array! { BTreeSet<T> where T: IntoLua + Ord }
impl_into_lua_array! { HashSet<T, S> where T: Eq + Hash + IntoLua, S: Default + BuildHasher }

/// Implements [`IntoLua`] for table types.
macro_rules! impl_into_lua_table {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoLua for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
                Ok(mlua::Value::Table(
                    lua.create_table_from(
                        self.into_iter()
                            .map(|(k, v)| {
                                Ok((IntoLua::into_lua(k, lua)?, IntoLua::into_lua(v, lua)?))
                            })
                            .collect::<Result<Vec<_>, LuaError>>()?,
                    )?
                ))
            }
        }
    }
}

impl_into_lua_table! { BTreeMap<K, V> where K: IntoLua + Ord, V: IntoLua }
impl_into_lua_table! {
    HashMap<K, V, S>
    where
        K: Eq + Hash + IntoLua,
        V: IntoLua,
        S: Default + BuildHasher
}

/// Implements [`IntoLua`] for serializable types.
macro_rules! impl_into_lua_serde {
    ($($ty:tt)*) => {
        impl IntoLua for $($ty)* {
            fn into_lua(self, lua: &Lua) -> Result<mlua::Value, LuaError> {
                use mlua::LuaSerdeExt;
                Ok(lua.to_value(&self)?)
            }
        }
    }
}

impl_into_lua_serde! { serde_json::Value }
impl_into_lua_serde! { toml::Value }
impl_into_lua_serde! { serde_yaml::Value }
