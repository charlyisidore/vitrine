//! Convert types from [`mlua::Value`].

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

/// Trait for types convertible from [`mlua::Value`].
pub(crate) trait FromLua: Sized {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> anyhow::Result<Self>;
}

impl FromLua for String {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> anyhow::Result<Self> {
        Ok(value
            .as_string()
            .ok_or_else(|| anyhow::anyhow!("Expected string, received {}", value.type_name()))?
            .to_str()?
            .to_owned())
    }
}

impl<T> FromLua for Option<T>
where
    T: FromLua,
{
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> anyhow::Result<Self> {
        if value.is_nil() {
            Ok(None)
        } else {
            Ok(Some(T::from_lua(value, lua)?))
        }
    }
}

impl<T> FromLua for Vec<T>
where
    T: FromLua,
{
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> anyhow::Result<Self> {
        value
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("Expected table, received {}", value.type_name()))?
            .to_owned()
            .sequence_values::<mlua::Value>()
            .map(|value| T::from_lua(value?, lua))
            .collect()
    }
}

impl<K, V, S> FromLua for HashMap<K, V, S>
where
    K: Eq + Hash + From<String>,
    V: FromLua,
    S: BuildHasher + Default,
{
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> anyhow::Result<Self> {
        value
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("Expected table, received {}", value.type_name()))?
            .to_owned()
            .pairs::<mlua::String, mlua::Value>()
            .map(|pair| {
                let (key, value): (mlua::String, mlua::Value) = pair?;
                let key = key.to_str()?;
                Ok((
                    K::from(key.to_owned()),
                    V::from_lua(value, lua)
                        .map_err(|error| error.context(format!("In field {}", key)))?,
                ))
            })
            .collect()
    }
}
