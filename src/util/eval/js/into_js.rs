//! Convert values into [`JsValue`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::{Path, PathBuf},
};

use super::{JsError, JsValue};

/// Trait for types convertible into [`JsValue`].
pub trait IntoJs
where
    Self: Sized,
{
    /// Perform the conversion.
    fn into_js(self) -> Result<JsValue, JsError>;
}

impl IntoJs for bool {
    fn into_js(self) -> Result<JsValue, JsError> {
        Ok(JsValue::Boolean(self))
    }
}

/// Implements [`IntoJs`] for signed integer types.
macro_rules! impl_into_js_int {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValue, JsError> {
                if <$ty>::BITS <= 32 || (self >= i32::MIN as $ty && self <= i32::MAX as $ty) {
                    Ok(JsValue::Int32(self as i32))
                } else {
                    Ok(JsValue::Number(self as f64))
                }
            }
        }
    };
}

impl_into_js_int! { i8 }
impl_into_js_int! { i16 }
impl_into_js_int! { i32 }
impl_into_js_int! { i64 }
impl_into_js_int! { i128 }
impl_into_js_int! { isize }

/// Implements [`IntoJs`] for unsigned integer types.
macro_rules! impl_into_js_uint {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValue, JsError> {
                if <$ty>::BITS <= 32 || self <= u32::MAX as $ty {
                    Ok(JsValue::Uint32(self as u32))
                } else {
                    Ok(JsValue::Number(self as f64))
                }
            }
        }
    };
}

impl_into_js_uint! { u8 }
impl_into_js_uint! { u16 }
impl_into_js_uint! { u32 }
impl_into_js_uint! { u64 }
impl_into_js_uint! { u128 }
impl_into_js_uint! { usize }

/// Implements [`IntoJs`] for number types.
macro_rules! impl_into_js_number {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValue, JsError> {
                Ok(JsValue::Number(self as f64))
            }
        }
    };
}

impl_into_js_number! { f32 }
impl_into_js_number! { f64 }

impl IntoJs for &str {
    fn into_js(self) -> Result<JsValue, JsError> {
        Ok(JsValue::String(self.to_string()))
    }
}

impl IntoJs for String {
    fn into_js(self) -> Result<JsValue, JsError> {
        Ok(JsValue::String(self))
    }
}

/// Implements [`IntoJs`] for path types.
macro_rules! impl_into_js_path {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValue, JsError> {
                let s = self.to_str().ok_or_else(|| JsError::IntoJs {
                    from: "Path",
                    to: "string",
                    message: Some("invalid unicode".to_string()),
                })?;
                Ok(JsValue::String(s.to_string()))
            }
        }
    };
}

impl_into_js_path! { &Path }
impl_into_js_path! { PathBuf }

/// Implements [`IntoJs`] for url types.
macro_rules! impl_into_js_url {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValue, JsError> {
                Ok(JsValue::String(self.into_string()))
            }
        }
    };
}

impl_into_js_url! { crate::util::url::Url }
impl_into_js_url! { crate::util::url::UrlPath }

impl<T> IntoJs for Option<T>
where
    T: IntoJs,
{
    fn into_js(self) -> Result<JsValue, JsError> {
        match self {
            Some(value) => T::into_js(value),
            None => Ok(JsValue::Null),
        }
    }
}

/// Implements [`IntoJs`] for array types.
macro_rules! impl_into_js_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoJs for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn into_js(self) -> Result<JsValue, JsError> {
                Ok(JsValue::Array(self
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, _>>()?))
            }
        }
    }
}

impl_into_js_array! { Vec<T> where T: IntoJs }
impl_into_js_array! { BTreeSet<T> where T: IntoJs + Ord }
impl_into_js_array! { HashSet<T, S> where T: Eq + Hash + IntoJs, S: Default + BuildHasher }

/// Implements [`IntoJs`] for object types.
macro_rules! impl_into_js_object {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(<$($v:ident),+>)? $(+ $tr:ident $(<$($w:ident),+>)?)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoJs for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(<$($v),+>)? $(+ $tr $(<$($w),+>)?)*),+
        )?
        {
            fn into_js(self) -> Result<JsValue, JsError> {
                Ok(JsValue::Object(self
                    .into_iter()
                    .map(|(k, v)| Ok((Into::into(k), IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?))
            }
        }
    }
}

impl_into_js_object! { BTreeMap<K, V> where K: Into<String> + Ord, V: IntoJs }
impl_into_js_object! {
    HashMap<K, V, S>
    where
        K: Eq + Hash + Into<String>,
        V: IntoJs,
        S: Default + BuildHasher
}

impl IntoJs for crate::util::value::Value {
    fn into_js(self) -> Result<JsValue, JsError> {
        match self {
            Self::Unit => Ok(JsValue::Null),
            Self::Bool(value) => Ok(JsValue::Boolean(value)),
            Self::I64(value) => Ok(JsValue::Number(value as f64)),
            Self::U64(value) => Ok(JsValue::Number(value as f64)),
            Self::F64(value) => Ok(JsValue::Number(value)),
            Self::Str(value) => Ok(JsValue::String(value)),
            Self::Seq(value) => Ok(JsValue::Array(
                value
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, _>>()?,
            )),
            Self::Map(value) => Ok(JsValue::Object(
                value
                    .into_iter()
                    .map(|(k, v)| Ok((k, IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?,
            )),
        }
    }
}

impl IntoJs for serde_json::Value {
    fn into_js(self) -> Result<JsValue, JsError> {
        match self {
            Self::Null => Ok(JsValue::Null),
            Self::Bool(value) => Ok(JsValue::Boolean(value)),
            Self::Number(value) => {
                if value.is_i64() {
                    Ok(JsValue::Number(value.as_i64().unwrap() as f64))
                } else if value.is_u64() {
                    Ok(JsValue::Number(value.as_u64().unwrap() as f64))
                } else if value.is_f64() {
                    Ok(JsValue::Number(value.as_f64().unwrap()))
                } else {
                    unreachable!()
                }
            },
            Self::String(value) => Ok(JsValue::String(value)),
            Self::Array(value) => Ok(JsValue::Array(
                value
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, _>>()?,
            )),
            Self::Object(value) => Ok(JsValue::Object(
                value
                    .into_iter()
                    .map(|(k, v)| Ok((k, IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?,
            )),
        }
    }
}
