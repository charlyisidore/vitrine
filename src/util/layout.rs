//! Render layouts.
//!
//! This module provides the [`LayoutEngine`] trait to unify layout engine APIs.

#[cfg(feature = "jinja")]
pub mod jinja;
pub mod multi;
#[cfg(feature = "tera")]
pub mod tera;

use serde::Serialize;

use crate::util::{function::Function, value::Value};

/// Type used for maps.
pub type Map<K, V> = std::collections::HashMap<K, V>;

/// Custom filter for layout engines.
///
/// This type unifies the signature for all layout engines.
///
/// - Jinja: `Fn(Value, *args, **kwargs) -> Value`
/// - Liquid: `Fn(Value, *args, **kwargs) -> Value`
/// - Tera: `Fn(&Value, &HashMap<String, Value>) -> Value`
pub type LayoutFilter = Function<(Value, Vec<Value>, Map<String, Value>), Value>;

/// Custom function for the layout engine.
///
/// This type unifies the signature for all layout engines.
///
/// - Jinja: `Fn(*args, **kwargs) -> Value`
/// - Liquid: not applicable
/// - Tera: `Fn(&HashMap<String, Value>) -> Value`
pub type LayoutFunction = Function<(Vec<Value>, Map<String, Value>), Value>;

/// Custom test for the layout engine.
///
/// This type unifies the signature for all layout engines.
///
/// - Jinja: `Fn(Value, *args, **kwargs) -> bool`
/// - Liquid: not applicable
/// - Tera: `Fn(Option<&Value>, &[Value]) -> bool`
pub type LayoutTest = Function<(Value, Vec<Value>, Map<String, Value>), bool>;

/// A trait for layout engines.
pub trait LayoutEngine {
    /// Error type.
    type Error: std::error::Error + 'static;

    /// Add layouts from an iterator of `(name, source)` pairs.
    fn add_layouts(
        &mut self,
        iter: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<(), Self::Error>;

    /// Add a custom filter.
    fn add_filter(&mut self, name: impl AsRef<str>, f: LayoutFilter) -> Result<(), Self::Error>;

    /// Add a custom function.
    fn add_function(&mut self, name: impl AsRef<str>, f: LayoutFunction)
        -> Result<(), Self::Error>;

    /// Add a custom test.
    fn add_test(&mut self, name: impl AsRef<str>, f: LayoutTest) -> Result<(), Self::Error>;

    /// Render a layout by name with context.
    fn render(&self, name: impl AsRef<str>, context: impl Serialize)
        -> Result<String, Self::Error>;
}
