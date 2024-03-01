//! Read values from JavaScript scripts.

pub mod from_js;
pub mod into_js;

use std::{path::Path, sync::Arc};

pub use from_js::FromJs;
pub use into_js::IntoJs;
use quickjs_runtime::{builder::QuickJsRuntimeBuilder, jsutils::Script};
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum JsError {
    /// Error converting values from JavaScript.
    #[error("{}", format_from_js_error(.from, .to, .message))]
    FromJs {
        /// Source type name.
        from: String,
        /// Target type name.
        to: &'static str,
        /// Optional message.
        message: Option<String>,
    },
    /// Error converting values to JavaScript.
    #[error("{}", format_into_js_error(.from, .to, .message))]
    IntoJs {
        /// Source type name.
        from: &'static str,
        /// Target type name.
        to: &'static str,
        /// Optional message.
        message: Option<String>,
    },
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JavaScript runtime error.
    #[error(transparent)]
    QuickjsRuntime(#[from] quickjs_runtime::jsutils::JsError),
    /// Serde error.
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    /// Add a field name to the error context.
    #[error("field `{field}`")]
    WithField {
        /// Source error.
        source: Box<Self>,
        /// Field name.
        field: String,
    },
}

/// Read value from a JavaScript script file.
pub fn from_file<T>(path: impl AsRef<Path>) -> Result<T, JsError>
where
    T: FromJs,
{
    let path = path.as_ref();

    let source = std::fs::read_to_string(path)?;

    let runtime = Arc::new(QuickJsRuntimeBuilder::new().build());

    let path = path.to_string_lossy();
    let script = Script::new(&path, &source);
    let value = runtime.eval_sync(None, script)?;

    T::from_js(value, &runtime)
}

/// Read value from a JavaScript script string.
pub fn from_str<T>(source: impl AsRef<str>) -> Result<T, JsError>
where
    T: FromJs,
{
    let source = source.as_ref();

    let runtime = Arc::new(QuickJsRuntimeBuilder::new().build());

    let script = Script::new("", source);
    let value = runtime.eval_sync(None, script)?;

    T::from_js(value, &runtime)
}

/// Format [`JsError::FromJs`] error.
///
/// Add parentheses around `message` if not empty.
fn format_from_js_error(from: &String, to: &str, message: &Option<String>) -> String {
    let message = message
        .as_ref()
        .map_or_else(|| "".to_string(), |message| format!(" ({message})"));
    format!("error converting JS `{from}` to `{to}`{message}")
}

/// Format [`JsError::IntoJs`] error.
///
/// Add parentheses around `message` if not empty.
fn format_into_js_error(from: &str, to: &str, message: &Option<String>) -> String {
    let message = message
        .as_ref()
        .map_or_else(|| "".to_string(), |message| format!(" ({message})"));
    format!("error converting `{from}` to JS `{to}`{message}")
}

#[cfg(test)]
mod tests {
    use vitrine_derive::FromJs;

    use super::from_str;

    #[test]
    fn bool_from_js() {
        let value: bool = from_str("true").unwrap();
        assert_eq!(value, true);
    }

    #[test]
    fn f64_from_js() {
        let value: f64 = from_str("3.14").unwrap();
        assert_eq!(value, 3.14);
    }

    #[test]
    fn i32_from_js() {
        let value: i32 = from_str("-42").unwrap();
        assert_eq!(value, -42);
    }

    #[test]
    fn u32_from_js() {
        let value: u32 = from_str("42").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn string_from_js() {
        let value: String = from_str(r#""foo""#).unwrap();
        assert_eq!(value, "foo");
    }

    #[test]
    fn path_buf_from_js() {
        use std::path::PathBuf;
        let value: PathBuf = from_str(r#""foo""#).unwrap();
        assert_eq!(value, PathBuf::from("foo"));
    }

    #[test]
    fn some_option_from_js() {
        let value: Option<i32> = from_str("42").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn none_option_from_js_null() {
        let value: Option<i32> = from_str("null").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn none_option_from_js_undefined() {
        let value: Option<i32> = from_str("undefined").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn vec_from_js() {
        let value: Vec<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, vec![1, 2, 3]);
    }

    #[test]
    fn b_tree_set_from_js() {
        use std::collections::BTreeSet;
        let value: BTreeSet<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn hash_set_from_js() {
        use std::collections::HashSet;
        let value: HashSet<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn b_tree_map_from_js() {
        use std::collections::BTreeMap;
        let value: BTreeMap<String, i32> = from_str("({ foo: 1, bar: 2, baz: 3 })").unwrap();
        assert_eq!(
            value,
            BTreeMap::from([
                ("foo".to_string(), 1),
                ("bar".to_string(), 2),
                ("baz".to_string(), 3)
            ])
        );
    }

    #[test]
    fn hash_map_from_js() {
        use std::collections::HashMap;
        let value: HashMap<String, i32> = from_str("({ foo: 1, bar: 2, baz: 3 })").unwrap();
        assert_eq!(
            value,
            HashMap::from([
                ("foo".to_string(), 1),
                ("bar".to_string(), 2),
                ("baz".to_string(), 3)
            ])
        );
    }

    #[test]
    fn value_from_js() {
        use crate::util::value::Value;
        let value: Value = from_str(
            r#"({
                bool: true,
                int: 1,
                float: 3.14,
                string: "bar",
                unit: null,
                vec: [1, 2, 3],
                map: { baz: 1 },
            })"#,
        )
        .unwrap();
        assert_eq!(
            value,
            Value::Map(
                [
                    ("bool".into(), Value::Bool(true)),
                    ("int".into(), Value::I64(1)),
                    ("float".into(), Value::F64(3.14)),
                    ("string".into(), Value::Str("bar".into())),
                    ("unit".into(), Value::Unit),
                    (
                        "vec".into(),
                        Value::Seq([Value::I64(1), Value::I64(2), Value::I64(3)].into()),
                    ),
                    (
                        "map".into(),
                        Value::Map([("baz".into(), Value::I64(1))].into()),
                    ),
                ]
                .into(),
            )
        );
    }

    #[test]
    fn function_from_js() {
        use crate::util::function::Function;
        let f: Function<(i32, i32), i32> = from_str("(x, y) => x + y").unwrap();
        assert_eq!(f.call(1, 2).unwrap(), 3);
    }

    #[test]
    fn derive_struct_from_js() {
        #[derive(FromJs)]
        struct Data {
            foo: String,
        }

        let result: Data = from_str(r#"({ foo: "bar" })"#).unwrap();
        assert_eq!(result.foo, "bar");
    }

    #[test]
    fn derive_struct_skip_from_js() {
        #[derive(FromJs)]
        struct Data {
            #[vitrine(skip)]
            foo: String,
        }

        let result: Data = from_str(r#"({ foo: "bar" })"#).unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_from_js() {
        #[derive(FromJs)]
        struct Data {
            #[vitrine(default)]
            foo: String,
        }

        let result: Data = from_str("({})").unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_fn_from_js() {
        #[derive(FromJs)]
        struct Data {
            #[vitrine(default = "default_foo")]
            foo: String,
        }

        fn default_foo() -> String {
            "bar".to_string()
        }

        let result: Data = from_str("({})").unwrap();
        assert_eq!(result.foo, "bar");
    }
}
