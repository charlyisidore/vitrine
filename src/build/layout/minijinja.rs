//! minijinja (Jinja) layout engine.
//!
//! This module uses [`minijinja`] under the hood.

use minijinja::Environment;
use serde::Serialize;
use thiserror::Error;

use super::{LayoutEngine, LayoutFilter, LayoutFunction, LayoutTest};

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum MinijinjaError {
    /// Minijinja error.
    #[error(transparent)]
    Minijinja(#[from] minijinja::Error),
}

/// Jinja layout engine.
#[derive(Debug)]
pub struct MinijinjaEngine<'source> {
    /// Minijinja environment.
    env: Environment<'source>,
}

impl<'source> MinijinjaEngine<'source> {
    /// Create a Jinja layout engine.
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    /// Return a reference to the [`Environment`] instance.
    pub fn env_ref(&mut self) -> &Environment {
        &self.env
    }

    /// Return a mutable reference to the [`Environment`] instance.
    pub fn env_mut(&mut self) -> &'source mut Environment {
        &mut self.env
    }
}

impl LayoutEngine for MinijinjaEngine<'_> {
    type Error = MinijinjaError;

    fn add_layouts(
        &mut self,
        iter: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<(), Self::Error> {
        for (name, source) in iter {
            self.env
                .add_template_owned(name.as_ref().to_string(), source.as_ref().to_string())?;
        }
        Ok(())
    }

    fn add_filter(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFilter>,
    ) -> Result<(), Self::Error> {
        use minijinja::{value::Rest, Error, ErrorKind, Value};
        let f = f.into();
        let f = move |value: Value, args: Rest<Value>| -> Result<Value, Error> {
            let value = crate::util::value::to_value(value)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))?;
            let args = args
                .iter()
                .map(crate::util::value::to_value)
                .collect::<Result<_, _>>()
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))?;
            let kwargs = Default::default();
            f.call(value, args, kwargs)
                .map(Value::from_serialize)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))
        };
        self.env.add_filter(name.as_ref().to_string(), f);
        Ok(())
    }

    fn add_function(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutFunction>,
    ) -> Result<(), Self::Error> {
        use minijinja::{value::Rest, Error, ErrorKind, Value};
        let f = f.into();
        let f = move |args: Rest<Value>| -> Result<Value, Error> {
            let args = args
                .iter()
                .map(crate::util::value::to_value)
                .collect::<Result<_, _>>()
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))?;
            let kwargs = Default::default();
            f.call(args, kwargs)
                .map(Value::from_serialize)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))
        };
        self.env.add_function(name.as_ref().to_string(), f);
        Ok(())
    }

    fn add_test(
        &mut self,
        name: impl AsRef<str>,
        f: impl Into<LayoutTest>,
    ) -> Result<(), Self::Error> {
        use minijinja::{value::Rest, Error, ErrorKind, Value};
        let f = f.into();
        let f = move |value: Value, args: Rest<Value>| -> Result<bool, Error> {
            let value = crate::util::value::to_value(value)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))?;
            let args = args
                .iter()
                .map(crate::util::value::to_value)
                .collect::<Result<_, _>>()
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))?;
            let kwargs = Default::default();
            f.call(value, args, kwargs)
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("{e}")))
        };
        self.env.add_test(name.as_ref().to_string(), f);
        Ok(())
    }

    fn render(
        &self,
        name: impl AsRef<str>,
        context: impl Serialize,
    ) -> Result<String, Self::Error> {
        let template = self.env.get_template(name.as_ref())?;
        Ok(template.render(context)?)
    }
}

impl Default for MinijinjaEngine<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::MinijinjaEngine;
    use crate::build::layout::{LayoutEngine, Value as LayoutValue};

    #[derive(Serialize)]
    struct Data {
        foo: String,
    }

    #[test]
    fn render_layouts() {
        let mut engine = MinijinjaEngine::new();

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
        let mut engine = MinijinjaEngine::new();

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
        let mut engine = MinijinjaEngine::new();

        let function = |args: Vec<LayoutValue>, _| -> LayoutValue {
            args.get(0)
                .and_then(|s| s.as_str())
                .map(|s| s.to_uppercase())
                .map_or_else(|| LayoutValue::Unit, |s| LayoutValue::Str(s))
        };

        engine.add_function("upper_case", function).unwrap();

        engine
            .add_layouts([("page", "{{ upper_case(foo) }}")])
            .unwrap();

        let context = Data {
            foo: "bar".to_string(),
        };

        let result = engine.render("page", context).unwrap();

        assert_eq!(result, "BAR");
    }

    #[test]
    fn custom_test() {
        let mut engine = MinijinjaEngine::new();

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
