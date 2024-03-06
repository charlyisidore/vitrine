//! Render layouts.
//!
//! This module provides the [`LayoutEngine`] trait to unify layout engine APIs.

#[cfg(feature = "jinja")]
pub mod jinja;
pub mod multi;
#[cfg(feature = "tera")]
pub mod tera;

use serde::Serialize;
use thiserror::Error;

use crate::util::{function::Function, value::Value};

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum LayoutError {
    /// Boxed error.
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error>),
}

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
    fn add_filter(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFilter>,
    ) -> Result<(), Self::Error>;

    /// Add a custom function.
    fn add_function(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error>;

    /// Add a custom test.
    fn add_test(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error>;

    /// Render a layout by name with context.
    fn render(&self, name: impl AsRef<str>, context: impl Serialize)
        -> Result<String, Self::Error>;
}

/// A trait that allows to store [`LayoutEngine`] in containers such as [`Vec`].
pub trait DynamicLayoutEngine {
    /// Calls [`LayoutEngine::add_layouts`].
    fn add_layouts(&mut self, layouts: &[(&str, &str)]) -> Result<(), LayoutError>;

    /// Calls [`LayoutEngine::add_filter`].
    fn add_filter(&mut self, name: &str, f: LayoutFilter) -> Result<(), LayoutError>;

    /// Calls [`LayoutEngine::add_function`].
    fn add_function(&mut self, name: &str, f: LayoutFunction) -> Result<(), LayoutError>;

    /// Calls [`LayoutEngine::add_test`].
    fn add_test(&mut self, name: &str, f: LayoutTest) -> Result<(), LayoutError>;

    /// Calls [`LayoutEngine::render`].
    fn render(&self, name: &str, context: Value) -> Result<String, LayoutError>;
}

impl<T> DynamicLayoutEngine for T
where
    T: LayoutEngine,
{
    fn add_layouts(&mut self, layouts: &[(&str, &str)]) -> Result<(), LayoutError> {
        self.add_layouts(layouts.iter().map(|v| v.to_owned()))
            .map_err(|e| LayoutError::Boxed(Box::new(e)))
    }

    fn add_filter(&mut self, name: &str, f: LayoutFilter) -> Result<(), LayoutError> {
        self.add_filter(name, f)
            .map_err(|e| LayoutError::Boxed(Box::new(e)))
    }

    fn add_function(&mut self, name: &str, f: LayoutFunction) -> Result<(), LayoutError> {
        self.add_function(name, f)
            .map_err(|e| LayoutError::Boxed(Box::new(e)))
    }

    fn add_test(&mut self, name: &str, f: LayoutTest) -> Result<(), LayoutError> {
        self.add_test(name, f)
            .map_err(|e| LayoutError::Boxed(Box::new(e)))
    }

    fn render(&self, name: &str, context: Value) -> Result<String, LayoutError> {
        self.render(name, context)
            .map_err(|e| LayoutError::Boxed(Box::new(e)))
    }
}

impl std::fmt::Debug for dyn DynamicLayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicLayoutEngine")
    }
}
