//! Multi layout engine.
//!
//! This module provides a layout engine that call sub-engines depending on the
//! layout file extension.

use std::collections::HashMap;

use serde::Serialize;
use thiserror::Error;

use super::{DynamicLayoutEngine, LayoutEngine, LayoutFilter, LayoutFunction, LayoutTest};

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MultiError {
    /// Layout error.
    #[error(transparent)]
    Layout(#[from] super::LayoutError),
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
#[derive(Debug, Default)]
pub struct MultiEngine {
    /// List of layout engines.
    engines: HashMap<String, Box<dyn DynamicLayoutEngine>>,
}

impl MultiEngine {
    /// Create a multi layout engine.
    pub fn new() -> Self {
        Self {
            engines: Default::default(),
        }
    }

    /// Add a layout engine.
    pub fn add_engine(
        &mut self,
        extension: impl AsRef<str>,
        engine: impl DynamicLayoutEngine + 'static,
    ) {
        self.engines
            .insert(extension.as_ref().to_string(), Box::new(engine));
    }

    /// Return a reference to the engine of given layout name.
    pub fn get_engine_ref(
        &self,
        name: impl AsRef<str>,
    ) -> Result<&(dyn DynamicLayoutEngine + 'static), MultiError> {
        use std::ops::Deref;

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
    pub fn get_engine_mut(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<&mut (dyn DynamicLayoutEngine + 'static), MultiError> {
        use std::ops::DerefMut;

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
        engine.add_filter(name, f.into()).map_err(Into::into)
    }

    fn add_function(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_mut(name)?;
        engine.add_function(name, f.into()).map_err(Into::into)
    }

    fn add_test(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_mut(name)?;
        engine.add_test(name, f.into()).map_err(Into::into)
    }

    fn render(
        &self,
        name: impl AsRef<str>,
        context: impl Serialize,
    ) -> Result<String, Self::Error> {
        let name = name.as_ref();
        let engine = self.get_engine_ref(name)?;
        let context = crate::util::value::to_value(context)?;
        engine.render(name, context).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    #[allow(unused_imports)]
    use super::MultiEngine;
    #[allow(unused_imports)]
    use crate::build::layout::LayoutEngine;

    #[derive(Serialize)]
    struct Data {
        foo: String,
    }

    #[cfg(feature = "minijinja")]
    #[test]
    fn render_jinja() {
        use crate::build::layout::minijinja::MinijinjaEngine;

        let mut engine = MultiEngine::new();

        engine.add_engine("jinja", MinijinjaEngine::new());

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
        use crate::build::layout::tera::TeraEngine;

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
