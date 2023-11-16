//! Convert types from [`rhai::Dynamic`].

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::Arc,
};

use rhai::{Dynamic, Engine, AST};

/// Trait for types convertible from [`Dynamic`].
pub(crate) trait FromRhai: Sized {
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self>;
}

impl FromRhai for bool {
    fn from_rhai(value: &Dynamic, _: Arc<Engine>, _: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .as_bool()
            .map_err(|error| anyhow::anyhow!("Expected bool, received {}", error))
    }
}

impl FromRhai for String {
    fn from_rhai(value: &Dynamic, _: Arc<Engine>, _: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .into_string()
            .map_err(|error| anyhow::anyhow!("Expected String, received {}", error))
    }
}

impl FromRhai for PathBuf {
    fn from_rhai(value: &Dynamic, _: Arc<Engine>, _: Arc<AST>) -> anyhow::Result<Self> {
        Ok(value
            .to_owned()
            .into_string()
            .map_err(|error| anyhow::anyhow!("Expected String, received {}", error))?
            .into())
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
            Ok(Some(T::from_rhai(value, engine, ast)?))
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
            .ok_or_else(|| anyhow::anyhow!("Expected Array, received {}", value.type_name()))?
            .into_iter()
            .map(|value| FromRhai::from_rhai(&value, Arc::clone(&engine), Arc::clone(&ast)))
            .collect()
    }
}

impl<K, V, S> FromRhai for HashMap<K, V, S>
where
    K: Eq + Hash + From<String>,
    V: FromRhai,
    S: BuildHasher + Default,
{
    fn from_rhai(value: &Dynamic, engine: Arc<Engine>, ast: Arc<AST>) -> anyhow::Result<Self> {
        value
            .to_owned()
            .try_cast::<rhai::Map>()
            .ok_or_else(|| anyhow::anyhow!("Expected Map, received {}", value.type_name(),))?
            .into_iter()
            .map(|(key, value)| {
                Ok((
                    key.to_string().into(),
                    FromRhai::from_rhai(&value, Arc::clone(&engine), Arc::clone(&ast))
                        .map_err(|error| error.context(format!("In field {}", key)))?,
                ))
            })
            .collect()
    }
}

impl FromRhai for serde_json::Value {
    fn from_rhai(value: &Dynamic, _: Arc<Engine>, _: Arc<AST>) -> anyhow::Result<Self> {
        Ok(rhai::serde::from_dynamic(value)?)
    }
}
