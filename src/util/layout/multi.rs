//! Multi layout engine.
//!
//! This module provides a layout engine that call sub-engines depending on the
//! layout file extension.

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::Serialize;
use thiserror::Error;

use super::{LayoutEngine, LayoutFilter, LayoutFunction, LayoutTest, Value as LayoutValue};

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MultiError {
    /// Boxed error.
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error>),
    /// Missing layout file extension error.
    #[error("missing file extension")]
    MissingFileExtension,
    /// Unsupported layout file extension error.
    #[error("unsupported file extension `{extension}`")]
    UnsupportedFileExtension {
        /// The file extension.
        extension: String,
    },
    /// Value error.
    #[error(transparent)]
    Value(#[from] crate::util::value::ValueError),
}

/// Multi layout engine.
///
/// A layout engine that calls sub-engines depending on the layout file
/// extension.
#[derive(Debug)]
pub struct MultiEngine {
    /// List of layout engines.
    engines: HashMap<String, Box<dyn DynamicEngine>>,
}

impl MultiEngine {
    /// Create a multi layout engine.
    pub fn new() -> Self {
        Self {
            engines: Default::default(),
        }
    }

    /// Add a layout engine.
    pub fn add_engine(&mut self, extension: impl AsRef<str>, engine: impl DynamicEngine + 'static) {
        self.engines
            .insert(extension.as_ref().to_string(), Box::new(engine));
    }

    /// Return a reference to the engine of given layout name.
    fn get_engine(
        &self,
        name: impl AsRef<str>,
    ) -> Result<&(dyn DynamicEngine + 'static), MultiError> {
        let name = name.as_ref();

        let Some((_, extension)) = name.rsplit_once('.') else {
            return Err(MultiError::MissingFileExtension);
        };

        let Some(engine) = self.engines.get(extension) else {
            return Err(MultiError::UnsupportedFileExtension {
                extension: extension.to_string(),
            });
        };

        Ok(engine.deref())
    }

    /// Return a mutable reference to the engine of given layout name.
    fn get_engine_mut(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<&mut (dyn DynamicEngine + 'static), MultiError> {
        let name = name.as_ref();

        let Some((_, extension)) = name.rsplit_once('.') else {
            return Err(MultiError::MissingFileExtension);
        };

        let Some(engine) = self.engines.get_mut(extension) else {
            return Err(MultiError::UnsupportedFileExtension {
                extension: extension.to_string(),
            });
        };

        Ok(engine.deref_mut())
    }
}

impl LayoutEngine for MultiEngine {
    type Error = MultiError;

    fn add_layouts(
        &mut self,
        iter: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<(), Self::Error> {
        let mut layouts: HashMap<String, Vec<(String, String)>> = self
            .engines
            .keys()
            .map(|k| (k.to_owned(), Vec::new()))
            .collect();

        for (name, source) in iter {
            let (name, source) = (name.as_ref(), source.as_ref());

            let Some((_, extension)) = name.rsplit_once('.') else {
                return Err(Self::Error::MissingFileExtension);
            };

            let Some(list) = layouts.get_mut(extension) else {
                return Err(Self::Error::UnsupportedFileExtension {
                    extension: extension.to_string(),
                });
            };

            list.push((name.to_string(), source.to_string()));
        }

        for (extension, list) in layouts {
            if list.is_empty() {
                continue;
            }

            let list: Vec<_> = list.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

            self.engines
                .get_mut(&extension)
                .expect("must have engine")
                .add_layouts(&list)?;
        }

        Ok(())
    }

    fn add_filter(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFilter>,
    ) -> Result<(), Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_mut(name)?;
        engine.add_filter(name, f.into())
    }

    fn add_function(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_mut(name)?;
        engine.add_function(name, f.into())
    }

    fn add_test(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_mut(name)?;
        engine.add_test(name, f.into())
    }

    fn render(
        &self,
        name: impl AsRef<str>,
        context: impl Serialize,
    ) -> Result<String, Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine(name)?;
        let context = crate::util::value::to_value(context)?;
        engine.render(name, context)
    }
}

impl Default for MultiEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait that allows to store [`LayoutEngine`] in containers such as
/// [`HashMap`].
pub trait DynamicEngine {
    /// Calls [`LayoutEngine::add_layouts`].
    fn add_layouts(&mut self, layouts: &[(&str, &str)]) -> Result<(), MultiError>;

    /// Calls [`LayoutEngine::add_filter`].
    fn add_filter(&mut self, name: &str, f: LayoutFilter) -> Result<(), MultiError>;

    /// Calls [`LayoutEngine::add_function`].
    fn add_function(&mut self, name: &str, f: LayoutFunction) -> Result<(), MultiError>;

    /// Calls [`LayoutEngine::add_test`].
    fn add_test(&mut self, name: &str, f: LayoutTest) -> Result<(), MultiError>;

    /// Calls [`LayoutEngine::render`].
    fn render(&self, name: &str, context: LayoutValue) -> Result<String, MultiError>;
}

impl<T> DynamicEngine for T
where
    T: LayoutEngine,
{
    fn add_layouts(&mut self, layouts: &[(&str, &str)]) -> Result<(), MultiError> {
        self.add_layouts(layouts.iter().map(|v| v.to_owned()))
            .map_err(|e| MultiError::Boxed(Box::new(e)))
    }

    fn add_filter(&mut self, name: &str, f: LayoutFilter) -> Result<(), MultiError> {
        self.add_filter(name, f)
            .map_err(|e| MultiError::Boxed(Box::new(e)))
    }

    fn add_function(&mut self, name: &str, f: LayoutFunction) -> Result<(), MultiError> {
        self.add_function(name, f)
            .map_err(|e| MultiError::Boxed(Box::new(e)))
    }

    fn add_test(&mut self, name: &str, f: LayoutTest) -> Result<(), MultiError> {
        self.add_test(name, f)
            .map_err(|e| MultiError::Boxed(Box::new(e)))
    }

    fn render(&self, name: &str, context: LayoutValue) -> Result<String, MultiError> {
        self.render(name, context)
            .map_err(|e| MultiError::Boxed(Box::new(e)))
    }
}

impl std::fmt::Debug for dyn DynamicEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LayoutEngine")
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::MultiEngine;
    use crate::util::layout::LayoutEngine;

    #[derive(Serialize)]
    struct Data {
        foo: String,
    }

    #[cfg(feature = "jinja")]
    #[test]
    fn render_jinja() {
        use crate::util::layout::jinja::JinjaEngine;

        let mut engine = MultiEngine::new();

        engine.add_engine("jinja", JinjaEngine::new());

        engine.add_layouts([("page.jinja", "{{ foo }}")]).unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page.jinja", context).unwrap();

        assert_eq!(result, "bar");
    }

    #[cfg(feature = "tera")]
    #[test]
    fn render_tera() {
        use crate::util::layout::tera::TeraEngine;

        let mut engine = MultiEngine::new();

        engine.add_engine("tera", TeraEngine::new());

        engine.add_layouts([("page.tera", "{{ foo }}")]).unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page.tera", context).unwrap();

        assert_eq!(result, "bar");
    }
}
