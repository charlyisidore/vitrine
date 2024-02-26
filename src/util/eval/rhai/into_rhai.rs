//! Convert values into [`rhai::Dynamic`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
    path::{Path, PathBuf},
};

use rhai::Dynamic;

/// Trait for types convertible into [`rhai::Dynamic`].
pub trait IntoRhai
where
    Self: Sized,
{
    fn into_rhai(self) -> Dynamic;
}

impl IntoRhai for bool {
    fn into_rhai(self) -> Dynamic {
        Dynamic::from_bool(self)
    }
}

/// Implements [`IntoRhai`] for int types.
macro_rules! impl_into_rhai_int {
    ($ty:ty) => {
        impl IntoRhai for $ty {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from_int(self as i64)
            }
        }
    };
}

impl_into_rhai_int! { i8 }
impl_into_rhai_int! { i16 }
impl_into_rhai_int! { i32 }
impl_into_rhai_int! { i64 }
impl_into_rhai_int! { i128 }
impl_into_rhai_int! { isize }
impl_into_rhai_int! { u8 }
impl_into_rhai_int! { u16 }
impl_into_rhai_int! { u32 }
impl_into_rhai_int! { u64 }
impl_into_rhai_int! { u128 }
impl_into_rhai_int! { usize }

/// Implements [`IntoRhai`] for float types.
macro_rules! impl_into_rhai_float {
    ($ty:ty) => {
        impl IntoRhai for $ty {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from_float(self as f64)
            }
        }
    };
}

impl_into_rhai_float! { f32 }
impl_into_rhai_float! { f64 }

impl IntoRhai for &str {
    fn into_rhai(self) -> Dynamic {
        Dynamic::from(self.to_string())
    }
}

impl IntoRhai for String {
    fn into_rhai(self) -> Dynamic {
        Dynamic::from(self)
    }
}

/// Implements [`IntoRhai`] for path types.
macro_rules! impl_into_rhai_path {
    ($ty:ty) => {
        impl IntoRhai for $ty {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from(self.to_string_lossy().to_string())
            }
        }
    };
}

impl_into_rhai_path! { &Path }
impl_into_rhai_path! { PathBuf }

impl<T> IntoRhai for Option<T>
where
    T: IntoRhai,
{
    fn into_rhai(self) -> Dynamic {
        match self {
            Some(value) => T::into_rhai(value),
            None => Dynamic::from(()),
        }
    }
}

/// Implements [`IntoRhai`] for array types.
macro_rules! impl_into_rhai_array {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(+ $tr:ident)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoRhai for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(+ $tr)*),+
        )?
        {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from_iter(self.into_iter().map(IntoRhai::into_rhai))
            }
        }
    }
}

impl_into_rhai_array! { Vec<T> where T: IntoRhai }
impl_into_rhai_array! { BTreeSet<T> where T: IntoRhai + Ord }
impl_into_rhai_array! { HashSet<T, S> where T: Eq + Hash + IntoRhai, S: Default + BuildHasher }

/// Implements [`IntoRhai`] for map types.
macro_rules! impl_into_rhai_map {
    (
        $ty:ident $(<$($t:ident),+>)?
        $(
            where
                $($u:ident: $tr0:ident $(<$($v:ident),+>)? $(+ $tr:ident $(<$($w:ident),+>)?)*),+
        )?
    ) => {
        impl $(<$($t),+>)? IntoRhai for $ty $(<$($t),+>)?
        $(
            where
                $($u: $tr0 $(<$($v),+>)? $(+ $tr $(<$($w),+>)?)*),+
        )?
        {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from_iter(
                    self.into_iter()
                        .map(|(k, v)| (IntoRhai::into_rhai(k), IntoRhai::into_rhai(v))),
                )
            }
        }
    }
}

impl_into_rhai_map! { BTreeMap<K, V> where K: IntoRhai + Ord, V: IntoRhai }
impl_into_rhai_map! {
    HashMap<K, V, S>
    where
        K: Eq + Hash + IntoRhai,
        V: IntoRhai,
        S: Default + BuildHasher
}

/// Implements [`IntoRhai`] for serializable types.
macro_rules! impl_into_rhai_serde {
    ($($ty:tt)*) => {
        impl IntoRhai for $($ty)* {
            fn into_rhai(self) -> Dynamic {
                Dynamic::from(self)
            }
        }
    }
}

impl_into_rhai_serde! { serde_json::Value }
impl_into_rhai_serde! { toml::Value }
impl_into_rhai_serde! { serde_yaml::Value }
