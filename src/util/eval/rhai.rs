//! Read values from Rhai scripts.

pub mod from_rhai;
pub mod into_rhai;

use std::{path::Path, sync::Arc};

pub use from_rhai::FromRhai;
pub use into_rhai::IntoRhai;
use rhai::Engine;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum RhaiError {
    #[error(transparent)]
    BoxRhaiEval(#[from] Box<rhai::EvalAltResult>),
    #[error("{}", format_from_rhai_error(.from, .to, .message))]
    FromRhai {
        from: &'static str,
        to: &'static str,
        message: Option<String>,
    },
    #[error(transparent)]
    RhaiEval(#[from] rhai::EvalAltResult),
    #[error(transparent)]
    RhaiParse(#[from] rhai::ParseError),
    #[error("field `{field}`")]
    WithField { source: Box<Self>, field: String },
}

/// Read value from a Rhai script file.
pub fn from_file<T, P>(path: P) -> Result<T, RhaiError>
where
    T: FromRhai,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let engine = Engine::new();
    let ast = engine.compile_file(path.to_path_buf())?;
    let result = engine.eval_ast(&ast)?;
    T::from_rhai(result, &Arc::new((engine, ast)))
}

/// Read value from a Rhai script string.
pub fn from_str<T, S>(s: S) -> Result<T, RhaiError>
where
    T: FromRhai,
    S: AsRef<str>,
{
    let s = s.as_ref();
    let engine = Engine::new();
    let ast = engine.compile(s)?;
    let result = engine.eval_ast(&ast)?;
    T::from_rhai(result, &Arc::new((engine, ast)))
}

/// Format [`RhaiError::FromRhai`] error.
///
/// Add parentheses around `message` if not empty.
fn format_from_rhai_error(from: &str, to: &str, message: &Option<String>) -> String {
    let message = message
        .as_ref()
        .map_or_else(|| "".to_string(), |message| format!(" ({message})"));
    format!("error converting Rhai `{from}` to `{to}`{message}")
}

#[cfg(test)]
mod tests {
    use vitrine_derive::FromRhai;

    use super::from_str;

    #[test]
    fn bool_from_rhai() {
        let value: bool = from_str("true").unwrap();
        assert_eq!(value, true);
    }

    #[test]
    fn f64_from_rhai() {
        let value: f64 = from_str("3.14").unwrap();
        assert_eq!(value, 3.14);
    }

    #[test]
    fn i32_from_rhai() {
        let value: i32 = from_str("-42").unwrap();
        assert_eq!(value, -42);
    }

    #[test]
    fn u32_from_rhai() {
        let value: u32 = from_str("42").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn string_from_rhai() {
        let value: String = from_str(r#""foo""#).unwrap();
        assert_eq!(value, "foo");
    }

    #[test]
    fn path_buf_from_rhai() {
        use std::path::PathBuf;
        let value: PathBuf = from_str(r#""foo""#).unwrap();
        assert_eq!(value, PathBuf::from("foo"));
    }

    #[test]
    fn some_option_from_rhai() {
        let value: Option<i32> = from_str("42").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn none_option_from_rhai() {
        let value: Option<i32> = from_str("()").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn vec_from_rhai() {
        let value: Vec<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, vec![1, 2, 3]);
    }

    #[test]
    fn b_tree_set_from_rhai() {
        use std::collections::BTreeSet;
        let value: BTreeSet<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn hash_set_from_rhai() {
        use std::collections::HashSet;
        let value: HashSet<i32> = from_str("[1, 2, 3]").unwrap();
        assert_eq!(value, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn b_tree_map_from_rhai() {
        use std::collections::BTreeMap;
        let value: BTreeMap<String, i32> = from_str("#{ foo: 1, bar: 2, baz: 3 }").unwrap();
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
    fn hash_map_from_rhai() {
        use std::collections::HashMap;
        let value: HashMap<String, i32> = from_str("#{ foo: 1, bar: 2, baz: 3 }").unwrap();
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
    fn function_from_rhai() {
        use crate::util::function::Function;
        let f: Function<(i32, i32), i32> = from_str("|x, y| x + y").unwrap();
        assert_eq!(f.call((1, 2)).unwrap(), 3);
    }

    #[test]
    fn derive_struct_from_rhai() {
        #[derive(FromRhai)]
        struct Data {
            foo: String,
        }

        let result: Data = from_str(r#"#{ foo: "bar" }"#).unwrap();
        assert_eq!(result.foo, "bar");
    }

    #[test]
    fn derive_struct_skip_from_rhai() {
        #[derive(FromRhai)]
        struct Data {
            #[vitrine(skip)]
            foo: String,
        }

        let result: Data = from_str(r#"#{ foo: "bar" }"#).unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_from_rhai() {
        #[derive(FromRhai)]
        struct Data {
            #[vitrine(default)]
            foo: String,
        }

        let result: Data = from_str("#{}").unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_fn_from_rhai() {
        #[derive(FromRhai)]
        struct Data {
            #[vitrine(default = "default_foo")]
            foo: String,
        }

        fn default_foo() -> String {
            "bar".to_string()
        }

        let result: Data = from_str("#{}").unwrap();
        assert_eq!(result.foo, "bar");
    }
}
