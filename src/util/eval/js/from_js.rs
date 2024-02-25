//! Convert values from [`JsValueFacade`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::Arc,
};

use quickjs_runtime::{facades::QuickJsRuntimeFacade, values::JsValueFacade};

use super::{into_js::IntoJs, JsError};
use crate::util::function::Function;

/// Trait for types convertible from [`JsValueFacade`].
pub trait FromJs
where
    Self: Sized,
{
    fn from_js(value: JsValueFacade, runtime: &Arc<QuickJsRuntimeFacade>) -> Result<Self, JsError>;
}

impl FromJs for bool {
    fn from_js(value: JsValueFacade, _: &Arc<QuickJsRuntimeFacade>) -> Result<Self, JsError> {
        if value.is_bool() {
            Ok(value.get_bool())
        } else {
            Err(JsError::FromJs {
                from: value.get_value_type().to_string(),
                to: "bool",
                message: Some("expected boolean".to_string()),
            })
        }
    }
}

/// Implements [`FromJs`] for float types.
macro_rules! impl_from_js_float {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(
                value: JsValueFacade,
                _: &Arc<QuickJsRuntimeFacade>,
            ) -> Result<Self, JsError> {
                if value.is_f64() {
                    Ok(value.get_f64() as $ty)
                } else {
                    Err(JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: stringify!($ty),
                        message: Some("expected number".to_string()),
                    })
                }
            }
        }
    };
}

impl_from_js_float! { f32 }
impl_from_js_float! { f64 }

/// Implements [`FromJs`] for integer types.
macro_rules! impl_from_js_integer {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(
                value: JsValueFacade,
                _: &Arc<QuickJsRuntimeFacade>,
            ) -> Result<Self, JsError> {
                if value.is_i32() {
                    Ok(value.get_i32() as $ty)
                } else {
                    Err(JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: stringify!($ty),
                        message: Some("expected number".to_string()),
                    })
                }
            }
        }
    };
}

impl_from_js_integer! { i8 }
impl_from_js_integer! { i16 }
impl_from_js_integer! { i32 }
impl_from_js_integer! { i64 }
impl_from_js_integer! { i128 }
impl_from_js_integer! { isize }
impl_from_js_integer! { u16 }
impl_from_js_integer! { u32 }
impl_from_js_integer! { u64 }
impl_from_js_integer! { u128 }
impl_from_js_integer! { usize }

/// Implements [`FromJs`] for string types.
macro_rules! impl_from_js_string {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(
                value: JsValueFacade,
                _: &Arc<QuickJsRuntimeFacade>,
            ) -> Result<Self, JsError> {
                if value.is_string() {
                    Ok(value.get_str().into())
                } else {
                    Err(JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: stringify!($ty),
                        message: Some("expected string".to_string()),
                    })
                }
            }
        }
    };
}

impl_from_js_string! { String }
impl_from_js_string! { PathBuf }

impl<T> FromJs for Option<T>
where
    T: FromJs,
{
    fn from_js(value: JsValueFacade, runtime: &Arc<QuickJsRuntimeFacade>) -> Result<Self, JsError> {
        if value.is_null_or_undefined() {
            Ok(None)
        } else {
            Ok(Some(T::from_js(value, runtime)?))
        }
    }
}

/// Implements [`FromJs`] for array types.
macro_rules! impl_from_js_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromJs for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn from_js(
                value: JsValueFacade,
                runtime: &Arc<QuickJsRuntimeFacade>,
            ) -> Result<Self, JsError> {
                match value {
                    JsValueFacade::Array { val } => val
                        .into_iter()
                        .map(|v| FromJs::from_js(v, runtime))
                        .collect(),
                    JsValueFacade::JsArray { cached_array } => {
                        futures::executor::block_on(cached_array.get_array())?
                            .into_iter()
                            .map(|v| FromJs::from_js(v, runtime))
                            .collect()
                    },
                    _ => Err(JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: stringify!($ty),
                        message: Some("expected array".to_string()),
                    }),
                }
            }
        }
    }
}

impl_from_js_array! { Vec<T> where T: FromJs }
impl_from_js_array! { BTreeSet<T> where T: FromJs + Ord }
impl_from_js_array! { HashSet<T, S> where T: Eq + FromJs + Hash, S: Default + BuildHasher }

/// Implements [`FromJs`] for object types.
macro_rules! impl_from_js_object {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(<$($v:ident),+>)? $(+ $tr:ident $(<$($w:ident),+>)?)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromJs for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(<$($v),+>)? $(+ $tr $(<$($w),+>)?)*),+
        )?
        {
            fn from_js(value: JsValueFacade, runtime: &Arc<QuickJsRuntimeFacade>)
                -> Result<Self, JsError>
            {
                match value {
                    JsValueFacade::JsObject { cached_object } => cached_object
                        .get_object_sync()
                        .map_err(Into::into)
                        .and_then(|object| {
                            object
                                .into_iter()
                                .map(|(key, value)| Ok((
                                    From::from(key),
                                    FromJs::from_js(value, runtime)?,
                                )))
                                .collect()
                        }),
                    _ => Err(JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: stringify!($ty),
                        message: Some("expected object".to_string()),
                    }),
                }
            }
        }
    }
}

impl_from_js_object! { BTreeMap<K, V> where K: From<String> + Ord, V: FromJs }
impl_from_js_object! {
    HashMap<K, V, S>
    where
        K: Eq + From<String> + Hash,
        V: FromJs,
        S: Default + BuildHasher
}

impl FromJs for serde_json::Value {
    fn from_js(
        value: JsValueFacade,
        _runtime: &Arc<QuickJsRuntimeFacade>,
    ) -> Result<Self, JsError> {
        use futures::executor::block_on;
        use serde_json::Value;

        match value {
            JsValueFacade::Null => Ok(Value::Null),
            JsValueFacade::Undefined => Ok(Value::Null),
            JsValueFacade::Boolean { val } => Ok(Value::from(val)),
            JsValueFacade::F64 { val } => Ok(Value::from(val)),
            JsValueFacade::I32 { val } => Ok(Value::from(val)),
            JsValueFacade::String { val } => Ok(Value::from(val.to_string())),
            JsValueFacade::Array { val } => Ok(Value::from_iter(
                val.into_iter()
                    .map(|v| FromJs::from_js(v, _runtime))
                    .collect::<Result<Vec<Value>, JsError>>()?,
            )),
            JsValueFacade::JsArray { cached_array } => {
                Ok(block_on(cached_array.get_serde_value())?)
            },
            JsValueFacade::Object { val } => Ok(Value::from_iter(
                val.into_iter()
                    .map(|(k, v)| Ok((k, FromJs::from_js(v, _runtime)?)))
                    .collect::<Result<HashMap<String, Value>, JsError>>()?,
            )),
            JsValueFacade::JsObject { cached_object } => {
                Ok(block_on(cached_object.get_serde_value())?)
            },
            JsValueFacade::JsPromise { ref cached_promise } => FromJs::from_js(
                cached_promise
                    .get_promise_result_sync()?
                    .map_err(|e| JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: "serde_json::Value",
                        message: Some(e.stringify()),
                    })?,
                _runtime,
            ),
            JsValueFacade::JsonStr { json } => Ok(serde_json::from_str(&json)?),
            JsValueFacade::SerdeValue { value } => Ok(value.clone()),
            _ => Err(JsError::FromJs {
                from: value.get_value_type().to_string(),
                to: "serde_json::Value",
                message: Some("expected JSON value".to_string()),
            }),
        }
    }
}

/// Implements [`FromJs`] for [`Function`].
macro_rules! impl_from_js_fn {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> FromJs for Function <($($ty,)*), R>
        where
            $($ty: IntoJs,)*
            R: FromJs,
        {
            fn from_js(value: JsValueFacade, runtime: &Arc<QuickJsRuntimeFacade>)
                -> Result<Self, JsError> {
                    match value {
                        JsValueFacade::JsFunction { cached_function } => {
                            let runtime = Arc::clone(&runtime);
                            Ok(Self::from(move |$($arg: $ty),*| {
                                let args = Vec::from([$($ty::into_js($arg)?,)*]);
                                let result = cached_function.invoke_function_sync(args)?;
                                R::from_js(result, &runtime)
                            }))
                        },
                        _ => Err(JsError::FromJs {
                            from: value.get_value_type().to_string(),
                            to: "Function",
                            message: Some("expected function".to_string()),
                        }),
                    }
                }
        }
    }
}

impl_from_js_fn! {}
impl_from_js_fn! { a1: A1 }
impl_from_js_fn! { a1: A1, a2: A2 }
impl_from_js_fn! { a1: A1, a2: A2, a3: A3 }
impl_from_js_fn! { a1: A1, a2: A2, a3: A3, a4: A4 }
impl_from_js_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5 }
impl_from_js_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6 }
