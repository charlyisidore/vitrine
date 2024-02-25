//! Convert values from [`rhai::Dynamic`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::Arc,
};

use rhai::{Dynamic, Engine, AST};

use super::{into_rhai::IntoRhai, RhaiError};
use crate::util::function::Function;

/// Trait for types convertible from [`rhai::Dynamic`].
pub trait FromRhai
where
    Self: Sized,
{
    fn from_rhai(value: Dynamic, runtime: &Arc<(Engine, AST)>) -> Result<Self, RhaiError>;
}

impl FromRhai for bool {
    fn from_rhai(value: Dynamic, _: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
        value.as_bool().map_err(|type_name| RhaiError::FromRhai {
            from: type_name,
            to: "bool",
            message: Some("expected bool".to_string()),
        })
    }
}

/// Implements [`FromRhai`] for float types.
macro_rules! impl_from_rhai_float {
    ($ty:ty) => {
        impl FromRhai for $ty {
            fn from_rhai(value: Dynamic, _: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                Ok(value.as_float().map_err(|type_name| RhaiError::FromRhai {
                    from: type_name,
                    to: stringify!($ty),
                    message: Some("expected float".to_string()),
                })? as $ty)
            }
        }
    };
}

impl_from_rhai_float! { f32 }
impl_from_rhai_float! { f64 }

/// Implements [`FromRhai`] for int types.
macro_rules! impl_from_rhai_int {
    ($ty:ty) => {
        impl FromRhai for $ty {
            fn from_rhai(value: Dynamic, _: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                Ok(value.as_int().map_err(|type_name| RhaiError::FromRhai {
                    from: type_name,
                    to: stringify!($ty),
                    message: Some("expected int".to_string()),
                })? as $ty)
            }
        }
    };
}

impl_from_rhai_int! { i8 }
impl_from_rhai_int! { i16 }
impl_from_rhai_int! { i32 }
impl_from_rhai_int! { i64 }
impl_from_rhai_int! { i128 }
impl_from_rhai_int! { isize }
impl_from_rhai_int! { u8 }
impl_from_rhai_int! { u16 }
impl_from_rhai_int! { u32 }
impl_from_rhai_int! { u64 }
impl_from_rhai_int! { u128 }
impl_from_rhai_int! { usize }

/// Implements [`FromRhai`] for string types.
macro_rules! impl_from_rhai_string {
    ($ty:ty) => {
        impl FromRhai for $ty {
            fn from_rhai(value: Dynamic, _: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                Ok(value
                    .into_string()
                    .map_err(|type_name| RhaiError::FromRhai {
                        from: type_name,
                        to: stringify!($ty),
                        message: Some("expected string".to_string()),
                    })?
                    .into())
            }
        }
    };
}

impl_from_rhai_string! { String }
impl_from_rhai_string! { PathBuf }

impl<T> FromRhai for Option<T>
where
    T: FromRhai,
{
    fn from_rhai(value: Dynamic, runtime: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
        if value.is_unit() {
            Ok(None)
        } else {
            Ok(Some(T::from_rhai(value, runtime)?))
        }
    }
}

/// Implements [`FromRhai`] for array types.
macro_rules! impl_from_rhai_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromRhai for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn from_rhai(value: Dynamic, runtime: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                value
                    .into_array()
                    .map_err(|type_name| RhaiError::FromRhai {
                        from: type_name,
                        to: stringify!($ty),
                        message: Some("expected array".to_string()),
                    })?
                    .into_iter()
                    .map(|v| FromRhai::from_rhai(v, runtime))
                    .collect()
            }
        }
    }
}

impl_from_rhai_array! { Vec<T> where T: FromRhai }
impl_from_rhai_array! { BTreeSet<T> where T: FromRhai + Ord }
impl_from_rhai_array! { HashSet<T, S> where T: Eq + FromRhai + Hash, S: Default + BuildHasher }

/// Implements [`FromRhai`] for map types.
macro_rules! impl_from_rhai_map {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(<$($v:ident),+>)? $(+ $tr:ident $(<$($w:ident),+>)?)*),+
        )?
    ) => {
        impl $(<$($t),+>)? FromRhai for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(<$($v),+>)? $(+ $tr $(<$($w),+>)?)*),+
        )?
        {
            fn from_rhai(value: Dynamic, runtime: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                let type_name = value.type_name();
                value
                    .try_cast::<rhai::Map>()
                    .ok_or_else(|| RhaiError::FromRhai {
                        from: type_name,
                        to: stringify!($ty),
                        message: Some("expected map".to_string()),
                    })?
                    .into_iter()
                    .map(|(k, v)| Ok((From::from(k.to_string()), FromRhai::from_rhai(v, runtime)?)))
                    .collect()
            }
        }
    }
}

impl_from_rhai_map! { BTreeMap<K, V> where K: From<String> + Ord, V: FromRhai }
impl_from_rhai_map! {
    HashMap<K, V, S>
    where
        K: Eq + From<String> + Hash,
        V: FromRhai,
        S: Default + BuildHasher
}

/// Implements [`FromRhai`] for deserializable types.
macro_rules! impl_from_rhai_serde {
    ($($ty:tt)*) => {
        impl FromRhai for $($ty)* {
            fn from_rhai(value: Dynamic, _: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                Ok(rhai::serde::from_dynamic(&value)?)
            }
        }
    }
}

impl_from_rhai_serde! { serde_json::Value }
impl_from_rhai_serde! { toml::Value }
impl_from_rhai_serde! { serde_yaml::Value }

/// Implements [`FromRhai`] for [`Function`].
macro_rules! impl_from_rhai_fn {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> FromRhai for Function <($($ty,)*), R>
        where
            $($ty: IntoRhai,)*
            R: FromRhai,
        {
            fn from_rhai(value: Dynamic, runtime: &Arc<(Engine, AST)>) -> Result<Self, RhaiError> {
                let type_name = value.type_name();
                let fn_ptr = value
                    .try_cast::<rhai::FnPtr>()
                    .ok_or_else(|| RhaiError::FromRhai {
                        from: type_name,
                        to: "Function",
                        message: Some("expected Fn".to_string()),
                    })?;
                let runtime = Arc::clone(runtime);

                Ok(Self::from(move |$($arg: $ty),*| {
                    let (engine, ast) = runtime.as_ref();
                    let args = ($($ty::into_rhai($arg),)*);
                    let result = fn_ptr.call(engine, ast, args)?;
                    R::from_rhai(result, &runtime)
                }))
            }
        }
    }
}

impl_from_rhai_fn! {}
impl_from_rhai_fn! { a1: A1 }
impl_from_rhai_fn! { a1: A1, a2: A2 }
impl_from_rhai_fn! { a1: A1, a2: A2, a3: A3 }
impl_from_rhai_fn! { a1: A1, a2: A2, a3: A3, a4: A4 }
impl_from_rhai_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5 }
impl_from_rhai_fn! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6 }
