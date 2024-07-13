//! Convert values from [`JsValue`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::PathBuf,
};

use super::{into_js::IntoJs, JsError, JsValue};
use crate::util::function::Function;

/// Trait for types convertible from [`JsValue`].
pub trait FromJs
where
    Self: Sized,
{
    /// Perform the conversion.
    fn from_js(value: JsValue) -> Result<Self, JsError>;
}

impl FromJs for bool {
    fn from_js(value: JsValue) -> Result<Self, JsError> {
        match value {
            JsValue::Boolean(v) => Ok(v),
            _ => Err(JsError::FromJs {
                from: value.type_str(),
                to: "bool",
                message: Some("expected boolean".to_string()),
            }),
        }
    }
}

/// Implements [`FromJs`] for integer types.
macro_rules! impl_from_js_integer {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::Number(v) if v.fract() == 0.0 => Ok(v as $ty),
                    JsValue::Int32(v) => Ok(v as $ty),
                    JsValue::Uint32(v) => Ok(v as $ty),
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
                        to: stringify!($ty),
                        message: Some("expected integer".to_string()),
                    }),
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
impl_from_js_integer! { u8 }
impl_from_js_integer! { u16 }
impl_from_js_integer! { u32 }
impl_from_js_integer! { u64 }
impl_from_js_integer! { u128 }
impl_from_js_integer! { usize }

/// Implements [`FromJs`] for number types.
macro_rules! impl_from_js_number {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::Number(v) => Ok(v as $ty),
                    JsValue::Int32(v) => Ok(v as $ty),
                    JsValue::Uint32(v) => Ok(v as $ty),
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
                        to: stringify!($ty),
                        message: Some("expected number".to_string()),
                    }),
                }
            }
        }
    };
}

impl_from_js_number! { f32 }
impl_from_js_number! { f64 }

/// Implements [`FromJs`] for string types.
macro_rules! impl_from_js_string {
    ($ty:ty) => {
        impl FromJs for $ty {
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::String(v) => Ok(v.into()),
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
                        to: stringify!($ty),
                        message: Some("expected string".to_string()),
                    }),
                }
            }
        }
    };
}

impl_from_js_string! { String }
impl_from_js_string! { PathBuf }
impl_from_js_string! { crate::util::url::Url }
impl_from_js_string! { crate::util::url::UrlPath }

impl<T> FromJs for Option<T>
where
    T: FromJs,
{
    fn from_js(value: JsValue) -> Result<Self, JsError> {
        match value {
            JsValue::Null | JsValue::Undefined => Ok(None),
            v => Ok(Some(T::from_js(v)?)),
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
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::Array(v) => v.into_iter().map(FromJs::from_js).collect(),
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
                        to: stringify!($ty),
                        message: Some("expected Array".to_string()),
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
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::Object(v) => v
                        .into_iter()
                        .map(|(k, v)| Ok((From::from(k), FromJs::from_js(v)?)))
                        .collect(),
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
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

impl FromJs for crate::util::value::Value {
    fn from_js(value: JsValue) -> Result<Self, JsError> {
        match value {
            JsValue::Null | JsValue::Undefined => Ok(Self::Unit),
            JsValue::Boolean(v) => Ok(Self::Bool(v)),
            JsValue::Number(v) => Ok(Self::F64(v)),
            JsValue::Int32(v) => Ok(Self::I64(v as i64)),
            JsValue::Uint32(v) => Ok(Self::U64(v as u64)),
            JsValue::String(v) => Ok(Self::Str(v)),
            JsValue::Array(v) => Ok(Self::Seq(
                v.into_iter().map(Self::from_js).collect::<Result<_, _>>()?,
            )),
            JsValue::Object(v) => Ok(Self::Map(
                v.into_iter()
                    .map(|(k, v)| Ok((k, Self::from_js(v)?)))
                    .collect::<Result<_, JsError>>()?,
            )),
            JsValue::Date(v) => Ok(Self::F64(v)),
            _ => Err(JsError::FromJs {
                from: value.type_str(),
                to: "vitrine::Value",
                message: Some("expected vitrine::Value".to_string()),
            }),
        }
    }
}

impl FromJs for serde_json::Value {
    fn from_js(value: JsValue) -> Result<Self, JsError> {
        use serde_json::Map;
        match value {
            JsValue::Null | JsValue::Undefined => Ok(Self::Null),
            JsValue::Boolean(v) => Ok(Self::from(v)),
            JsValue::Number(v) => Ok(Self::from(v)),
            JsValue::Int32(v) => Ok(Self::from(v)),
            JsValue::Uint32(v) => Ok(Self::from(v)),
            JsValue::String(v) => Ok(Self::from(v)),
            JsValue::Array(v) => Ok(Self::from_iter(
                v.into_iter()
                    .map(Self::from_js)
                    .collect::<Result<Vec<Self>, _>>()?,
            )),
            JsValue::Object(v) => Ok(Self::from_iter(
                v.into_iter()
                    .map(|(k, v)| Ok((k, Self::from_js(v)?)))
                    .collect::<Result<Map<String, Self>, JsError>>()?,
            )),
            JsValue::Date(v) => Ok(Self::from(v)),
            _ => Err(JsError::FromJs {
                from: value.type_str(),
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
            fn from_js(value: JsValue) -> Result<Self, JsError> {
                match value {
                    JsValue::Function(_, v) => {
                        Ok(Self::from(move |$($arg: $ty),*| {
                            let args = Vec::from([$($ty::into_js($arg)?,)*]);
                            let result = (v)(args)?;
                            R::from_js(result)
                        }))
                    },
                    _ => Err(JsError::FromJs {
                        from: value.type_str(),
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
