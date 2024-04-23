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
    Boxed(#[from] Box<dyn std::error::Error + Send + Sync>),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
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
    type Error: std::error::Error + Send + Sync + 'static;

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

/// Null layout engine.
///
/// A layout engine that does nothing.
#[derive(Debug, Default)]
pub struct NullEngine;

impl NullEngine {
    /// Create a null layout engine.
    pub fn new() -> Self {
        Self {}
    }
}

impl LayoutEngine for NullEngine {
    type Error = std::convert::Infallible;

    fn add_layouts(
        &mut self,
        _iter: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn add_filter(
        &mut self,
        _name: impl AsRef<str>,
        _f: impl Into<LayoutFilter>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn add_function(
        &mut self,
        _name: impl AsRef<str>,
        _f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn add_test(
        &mut self,
        _name: impl AsRef<str>,
        _f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn render(
        &self,
        _name: impl AsRef<str>,
        _context: impl Serialize,
    ) -> Result<String, Self::Error> {
        unimplemented!()
    }
}

/// Pipeline task.
pub mod task {
    #[cfg(feature = "jinja")]
    use super::jinja::JinjaEngine;
    #[cfg(feature = "tera")]
    use super::tera::TeraEngine;
    use super::{DynamicLayoutEngine, LayoutError, Map};
    use crate::{
        build::{input::DirWalker, Page},
        config::Config,
        util::pipeline::{Receiver, Sender, Task},
    };

    #[cfg(feature = "jinja")]
    type DefaultEngine<'source> = JinjaEngine<'source>;

    #[cfg(all(not(feature = "jinja"), feature = "tera"))]
    type DefaultEngine = TeraEngine;

    #[cfg(all(not(feature = "jinja"), not(feature = "tera")))]
    type DefaultEngine = super::NullEngine;

    /// Task to render layouts.
    #[derive(Debug)]
    pub struct LayoutTask<'config> {
        config: &'config Config,
        engine: Box<dyn DynamicLayoutEngine>,
    }

    impl<'config> LayoutTask<'config> {
        /// Create a pipeline task to render layouts.
        pub fn new(config: &'config Config) -> Result<Self, LayoutError> {
            let mut engine: Box<dyn DynamicLayoutEngine> =
                if let Some(engine) = &config.layout.engine {
                    match engine.as_str() {
                        #[cfg(feature = "jinja")]
                        "jinja" => Box::new(JinjaEngine::new()),
                        #[cfg(feature = "tera")]
                        "tera" => Box::new(TeraEngine::new()),
                        _ => Box::new(DefaultEngine::new()),
                    }
                } else {
                    Box::new(DefaultEngine::new())
                };

            if let Some(layout_dir) = &config.layout_dir {
                let layouts: Vec<(String, String)> = DirWalker::new(layout_dir)
                    .walk()
                    .map(|entry| -> Result<_, LayoutError> {
                        let name = entry
                            .path()
                            .strip_prefix(layout_dir)
                            .expect("path must start with `layout_dir`")
                            .to_string_lossy()
                            .to_string();
                        let source = std::fs::read_to_string(entry.path())?;
                        Ok((name, source))
                    })
                    .collect::<Result<_, _>>()?;

                let layouts: Vec<_> = layouts
                    .iter()
                    .map(|(name, source)| (name.as_str(), source.as_str()))
                    .collect();

                engine.add_layouts(&layouts)?;
            }

            for (name, f) in &config.layout.filters {
                engine.add_filter(name, f.clone())?;
            }

            for (name, f) in &config.layout.functions {
                engine.add_function(name, f.clone())?;
            }

            for (name, f) in &config.layout.tests {
                engine.add_test(name, f.clone())?;
            }

            Ok(Self { config, engine })
        }
    }

    impl Task<(Page,), (Page,), LayoutError> for LayoutTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (tx,): (Sender<Page>,),
        ) -> Result<(), LayoutError> {
            for page in rx {
                let Some(layout) = page.data.get("layout").and_then(|v| v.as_str()) else {
                    tx.send(page).unwrap();
                    continue;
                };

                let context = Map::from([
                    ("content".to_string(), page.content.to_string().into()),
                    ("url".to_string(), page.url.to_string().into()),
                    ("page".to_string(), page.data.clone()),
                    ("site".to_string(), self.config.site_data.clone()),
                ])
                .into();

                let content = self.engine.render(layout, context)?;

                tx.send(Page { content, ..page }).unwrap();
            }
            Ok(())
        }
    }
}
