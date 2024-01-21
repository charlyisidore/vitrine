//! Convert types from [`quickjs_runtime::JsValueFacade`].

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::Arc,
};

use quickjs_runtime::{facades::QuickJsRuntimeFacade, values::JsValueFacade};

/// Trait for types convertible from [`JsValueFacade`].
pub(crate) trait FromJs: Sized {
    fn from_js(value: JsValueFacade, runtime: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self>;
}

impl FromJs for bool {
    fn from_js(value: JsValueFacade, _: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        if value.is_bool() {
            Ok(value.get_bool())
        } else {
            Err(anyhow::anyhow!(
                "Expected boolean, received {}",
                value.get_value_type()
            ))
        }
    }
}

impl FromJs for f64 {
    fn from_js(value: JsValueFacade, _: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        if value.is_f64() {
            Ok(value.get_f64())
        } else {
            Err(anyhow::anyhow!(
                "Expected number, received {}",
                value.get_value_type()
            ))
        }
    }
}

impl FromJs for String {
    fn from_js(value: JsValueFacade, _: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        if value.is_string() {
            Ok(value.get_str().to_owned())
        } else {
            Err(anyhow::anyhow!(
                "Expected string, received {}",
                value.get_value_type()
            ))
        }
    }
}

impl FromJs for PathBuf {
    fn from_js(value: JsValueFacade, _: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        if value.is_string() {
            Ok(value.get_str().into())
        } else {
            Err(anyhow::anyhow!(
                "Expected string, received {}",
                value.get_value_type()
            ))
        }
    }
}

impl<T> FromJs for Option<T>
where
    T: FromJs,
{
    fn from_js(value: JsValueFacade, runtime: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        if value.is_null_or_undefined() {
            Ok(None)
        } else {
            Ok(Some(T::from_js(value, runtime)?))
        }
    }
}

impl<T> FromJs for Vec<T>
where
    T: FromJs,
{
    fn from_js(value: JsValueFacade, runtime: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        match value {
            JsValueFacade::Null | JsValueFacade::Undefined => Ok(Vec::default()),
            JsValueFacade::JsArray { cached_array } => {
                futures::executor::block_on(cached_array.get_array())?
                    .into_iter()
                    .map(|value| T::from_js(value, Arc::clone(&runtime)))
                    .collect()
            },
            _ => Err(anyhow::anyhow!(
                "Expected array, received {}",
                value.get_value_type()
            )),
        }
    }
}

impl<K, V, S> FromJs for HashMap<K, V, S>
where
    K: Eq + Hash + From<String>,
    V: FromJs,
    S: BuildHasher + Default,
{
    fn from_js(value: JsValueFacade, runtime: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        match value {
            JsValueFacade::Null | JsValueFacade::Undefined => Ok(HashMap::default()),
            JsValueFacade::JsObject { cached_object } => cached_object
                .get_object_sync()
                .map_err(|error| error.into())
                .and_then(|object| {
                    object
                        .into_iter()
                        .map(|(key, value)| {
                            Ok((
                                K::from(key.to_owned()),
                                V::from_js(value, Arc::clone(&runtime))
                                    .map_err(|error| error.context(format!("In field {}", key)))?,
                            ))
                        })
                        .collect()
                }),
            _ => Err(anyhow::anyhow!(
                "Expected object, received {}",
                value.get_value_type()
            )),
        }
    }
}

impl FromJs for serde_json::Value {
    fn from_js(value: JsValueFacade, _: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        futures::executor::block_on(value.to_serde_value()).map_err(|error| error.into())
    }
}
