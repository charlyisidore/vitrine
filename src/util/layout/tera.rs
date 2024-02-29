//! Tera layout engine.
//!
//! This module uses [`tera`] under the hood.

use std::collections::HashMap;

use serde::Serialize;
use tera::Tera;
use thiserror::Error;

use super::{LayoutEngine, LayoutFilter, LayoutFunction, LayoutTest};

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum TeraError {
    /// Tera error.
    #[error(transparent)]
    Tera(#[from] tera::Error),
}

/// Tera layout engine.
#[derive(Debug)]
pub struct TeraEngine {
    /// Tera engine.
    tera: Tera,
}

impl TeraEngine {
    /// Create a Tera layout engine.
    pub fn new() -> Self {
        Self {
            tera: Tera::default(),
        }
    }

    /// Return a reference to the [`Tera`] instance.
    pub fn tera_ref(&mut self) -> &Tera {
        &self.tera
    }

    /// Return a mutable reference to the [`Tera`] instance.
    pub fn tera_mut(&mut self) -> &mut Tera {
        &mut self.tera
    }
}

impl LayoutEngine for TeraEngine {
    type Error = TeraError;

    fn add_layouts(
        &mut self,
        iter: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<(), Self::Error> {
        Ok(self.tera.add_raw_templates(iter)?)
    }

    fn add_filter(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFilter>,
    ) -> Result<(), Self::Error> {
        use tera::{Error, Value};
        let f = f.into();
        let f = move |value: &Value, kwargs: &HashMap<String, Value>| -> Result<Value, Error> {
            let value =
                crate::util::value::to_value(value).map_err(|e| Error::msg(format!("{e}")))?;
            let args = Default::default();
            let kwargs = kwargs
                .iter()
                .map(|(k, v)| Ok((k.to_owned(), tera::from_value(v.to_owned())?)))
                .collect::<Result<_, serde_json::Error>>()
                .map_err(|e| Error::msg(format!("{e}")))?;
            f.call(value, args, kwargs)
                .map_err(|e| Error::msg(format!("{e}")))
                .and_then(|output| tera::to_value(output).map_err(|e| Error::msg(format!("{e}"))))
        };
        self.tera.register_filter(name.as_ref(), f);
        Ok(())
    }

    fn add_function(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error> {
        use tera::{Error, Value};
        let f = f.into();
        let f = move |kwargs: &HashMap<String, Value>| -> Result<Value, Error> {
            let args = Default::default();
            let kwargs = kwargs
                .iter()
                .map(|(k, v)| Ok((k.to_owned(), tera::from_value(v.to_owned())?)))
                .collect::<Result<_, serde_json::Error>>()
                .map_err(|e| Error::msg(format!("{e}")))?;
            f.call(args, kwargs)
                .map_err(|e| Error::msg(format!("{e}")))
                .and_then(|output| tera::to_value(output).map_err(|e| Error::msg(format!("{e}"))))
        };
        self.tera.register_function(name.as_ref(), f);
        Ok(())
    }

    fn add_test(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error> {
        use tera::{Error, Value};
        let f = f.into();
        let f = move |value: Option<&Value>, args: &[Value]| -> Result<bool, Error> {
            let value =
                crate::util::value::to_value(value).map_err(|e| Error::msg(format!("{e}")))?;
            let args = args
                .iter()
                .map(|v| tera::from_value(v.to_owned()))
                .collect::<Result<_, serde_json::Error>>()
                .map_err(|e| Error::msg(format!("{e}")))?;
            let kwargs = Default::default();
            f.call(value, args, kwargs)
                .map_err(|e| Error::msg(format!("{e}")))
        };
        self.tera.register_tester(name.as_ref(), f);
        Ok(())
    }

    fn render(
        &self,
        name: impl AsRef<str>,
        context: impl Serialize,
    ) -> Result<String, Self::Error> {
        let context = tera::Context::from_serialize(context)?;
        Ok(self.tera.render(name.as_ref(), &context)?)
    }
}

impl Default for TeraEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::TeraEngine;
    use crate::util::layout::{LayoutEngine, Map, Value as LayoutValue};

    #[derive(Serialize)]
    struct Data {
        foo: String,
    }

    #[test]
    fn render_layouts() {
        let mut engine = TeraEngine::new();

        engine
            .add_layouts([
                ("base", "{% block body %}{{ foo }}{% endblock %}"),
                (
                    "page",
                    r#"{% extends "base" %}{% block body %}{{ super() }}baz{% endblock %}"#,
                ),
            ])
            .unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page", context).unwrap();

        assert_eq!(result, "barbaz");
    }

    #[test]
    fn custom_filter() {
        let mut engine = TeraEngine::new();

        let filter = |value: LayoutValue, _, _| -> LayoutValue {
            value
                .as_str()
                .map(|s| s.to_uppercase())
                .map_or_else(|| LayoutValue::Unit, |s| LayoutValue::Str(s))
        };

        engine.add_filter("upper_case", filter).unwrap();

        engine
            .add_layouts([("page", "{{ foo | upper_case }}")])
            .unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page", context).unwrap();

        assert_eq!(result, "BAR");
    }

    #[test]
    fn custom_function() {
        let mut engine = TeraEngine::new();

        let function = |_, kwargs: Map<String, LayoutValue>| -> LayoutValue {
            kwargs
                .get("s")
                .and_then(|s| s.as_str())
                .map(|s| s.to_uppercase())
                .map_or_else(|| LayoutValue::Unit, |s| LayoutValue::Str(s))
        };

        engine.add_function("upper_case", function).unwrap();

        engine
            .add_layouts([("page", "{{ upper_case(s=foo) }}")])
            .unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page", context).unwrap();

        assert_eq!(result, "BAR");
    }

    #[test]
    fn custom_test() {
        let mut engine = TeraEngine::new();

        let test = |value: LayoutValue, _, _| -> bool {
            value
                .as_str()
                .map(|s| s == s.to_uppercase())
                .unwrap_or(false)
        };

        engine.add_test("upper_case", test).unwrap();

        engine
            .add_layouts([("page", "{{ foo is upper_case }}")])
            .unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page", context).unwrap();

        assert_eq!(result, "false");
    }
}
