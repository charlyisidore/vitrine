//! Convert values into [`JsValueFacade`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::{Path, PathBuf},
};

use quickjs_runtime::values::JsValueFacade;

use super::JsError;

/// Trait for types convertible into [`JsValueFacade`].
pub trait IntoJs
where
    Self: Sized,
{
    fn into_js(self) -> Result<JsValueFacade, JsError>;
}

impl IntoJs for bool {
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        Ok(JsValueFacade::new_bool(self))
    }
}

/// Implements [`IntoJs`] for integer types that fit in [`i32`].
macro_rules! impl_into_js_integer {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                Ok(JsValueFacade::new_i32(self as i32))
            }
        }
    };
}

impl_into_js_integer! { i8 }
impl_into_js_integer! { i16 }
impl_into_js_integer! { i32 }
impl_into_js_integer! { u8 }
impl_into_js_integer! { u16 }

/// Implements [`IntoJs`] for signed integer types larger than [`i32`].
macro_rules! impl_into_js_integer_signed {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                if self >= i32::MIN as $ty && self <= i32::MAX as $ty {
                    Ok(JsValueFacade::new_i32(self as i32))
                } else {
                    Ok(JsValueFacade::new_f64(self as f64))
                }
            }
        }
    };
}

impl_into_js_integer_signed! { i64 }
impl_into_js_integer_signed! { i128 }
impl_into_js_integer_signed! { isize }

/// Implements [`IntoJs`] for unsigned integer types larger than [`i32`].
macro_rules! impl_into_js_integer_unsigned {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                if self <= i32::MAX as $ty {
                    Ok(JsValueFacade::new_i32(self as i32))
                } else {
                    Ok(JsValueFacade::new_f64(self as f64))
                }
            }
        }
    };
}

impl_into_js_integer_unsigned! { u32 }
impl_into_js_integer_unsigned! { u64 }
impl_into_js_integer_unsigned! { u128 }
impl_into_js_integer_unsigned! { usize }

/// Implements [`IntoJs`] for float types.
macro_rules! impl_into_js_float {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                Ok(JsValueFacade::new_f64(self as f64))
            }
        }
    };
}

impl_into_js_float! { f32 }
impl_into_js_float! { f64 }

impl IntoJs for &str {
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        Ok(JsValueFacade::new_str(self))
    }
}

impl IntoJs for String {
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        Ok(JsValueFacade::new_string(self))
    }
}

/// Implements [`IntoJs`] for path types.
macro_rules! impl_into_js_path {
    ($ty:ty) => {
        impl IntoJs for $ty {
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                let s = self.to_str().ok_or_else(|| JsError::IntoJs {
                    from: "Path",
                    to: "string",
                    message: Some("invalid unicode".to_string()),
                })?;
                Ok(JsValueFacade::new_str(s))
            }
        }
    };
}

impl_into_js_path! { &Path }
impl_into_js_path! { PathBuf }

impl<T> IntoJs for Option<T>
where
    T: IntoJs,
{
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        match self {
            Some(value) => T::into_js(value),
            None => Ok(JsValueFacade::Null),
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
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                let val = self
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, _>>()?;
                Ok(JsValueFacade::Array { val })
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
            fn into_js(self) -> Result<JsValueFacade, JsError> {
                let val = self
                    .into_iter()
                    .map(|(k, v)| Ok((Into::into(k), IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?;
                Ok(JsValueFacade::Object { val })
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
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        match self {
            Self::Unit => Ok(JsValueFacade::Null),
            Self::Bool(value) => Ok(JsValueFacade::new_bool(value)),
            Self::I64(value) => {
                if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
                    Ok(JsValueFacade::new_i32(value as i32))
                } else {
                    Ok(JsValueFacade::new_f64(value as f64))
                }
            },
            Self::U64(value) => {
                if value <= i32::MAX as u64 {
                    Ok(JsValueFacade::new_i32(value as i32))
                } else {
                    Ok(JsValueFacade::new_f64(value as f64))
                }
            },
            Self::F64(value) => Ok(JsValueFacade::new_f64(value)),
            Self::Str(value) => Ok(JsValueFacade::new_string(value)),
            Self::Seq(value) => Ok(JsValueFacade::Array {
                val: value
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, JsError>>()?,
            }),
            Self::Map(value) => Ok(JsValueFacade::Object {
                val: value
                    .into_iter()
                    .map(|(k, v)| Ok((k, IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?,
            }),
        }
    }
}

impl IntoJs for serde_json::Value {
    fn into_js(self) -> Result<JsValueFacade, JsError> {
        match self {
            Self::Null => Ok(JsValueFacade::Null),
            Self::Bool(value) => Ok(JsValueFacade::new_bool(value)),
            Self::Number(value) => {
                if value.is_i64() {
                    let value = value.as_i64().unwrap();
                    if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
                        Ok(JsValueFacade::new_i32(value as i32))
                    } else {
                        Ok(JsValueFacade::new_f64(value as f64))
                    }
                } else if value.is_u64() {
                    let value = value.as_u64().unwrap();
                    if value <= i32::MAX as u64 {
                        Ok(JsValueFacade::new_i32(value as i32))
                    } else {
                        Ok(JsValueFacade::new_f64(value as f64))
                    }
                } else if value.is_f64() {
                    Ok(JsValueFacade::new_f64(value.as_f64().unwrap()))
                } else {
                    unreachable!()
                }
            },
            Self::String(value) => Ok(JsValueFacade::new_string(value)),
            Self::Array(value) => Ok(JsValueFacade::Array {
                val: value
                    .into_iter()
                    .map(IntoJs::into_js)
                    .collect::<Result<_, JsError>>()?,
            }),
            Self::Object(value) => Ok(JsValueFacade::Object {
                val: value
                    .into_iter()
                    .map(|(k, v)| Ok((k, IntoJs::into_js(v)?)))
                    .collect::<Result<_, JsError>>()?,
            }),
        }
    }
}
