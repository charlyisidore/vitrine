//! Read values from Lua scripts.

pub mod from_lua;
pub mod into_lua;

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub use from_lua::FromLua;
pub use into_lua::IntoLua;
use mlua::Lua;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum LuaError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Lua runtime error.
    #[error(transparent)]
    Lua(#[from] mlua::Error),
    /// Add a field name to the error context.
    #[error("field `{field}`")]
    WithField {
        /// Source error.
        source: Box<Self>,
        /// Field name.
        field: String,
    },
}

/// Read value from a Lua script file.
pub fn from_file<T>(path: impl AsRef<Path>) -> Result<T, LuaError>
where
    T: FromLua,
{
    from_str(std::fs::read_to_string(path)?)
}

/// Read value from a Lua script string.
pub fn from_str<T>(source: impl AsRef<str>) -> Result<T, LuaError>
where
    T: FromLua,
{
    let source = source.as_ref();
    let lua = new_lua();
    let lua = lua.lock().unwrap();
    let value = lua.load(source).eval()?;
    T::from_lua(value, &lua)
}

/// Create a Lua instance wrapped by a mutex.
fn new_lua() -> Arc<Mutex<Lua>> {
    // Call `unsafe_new()` to allow loading C modules
    let lua = unsafe { Lua::unsafe_new() };

    // `Lua` is `Send` but not `Sync`, we wrap it in `Arc<Mutex>`
    let lua = Arc::new(Mutex::new(lua));

    // Save `Lua` mutex in app data, retrieve it with `lua.app_data_ref()`
    lua.lock().unwrap().set_app_data(Arc::clone(&lua));

    lua
}

#[cfg(test)]
mod tests {
    use vitrine_derive::FromLua;

    use super::from_str;

    #[test]
    fn bool_from_lua() {
        let value: bool = from_str("true").unwrap();
        assert_eq!(value, true);
    }

    #[test]
    fn f64_from_lua() {
        let value: f64 = from_str("3.14").unwrap();
        assert_eq!(value, 3.14);
    }

    #[test]
    fn i32_from_lua() {
        let value: i32 = from_str("-42").unwrap();
        assert_eq!(value, -42);
    }

    #[test]
    fn u32_from_lua() {
        let value: u32 = from_str("42").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn string_from_lua() {
        let value: String = from_str(r#""foo""#).unwrap();
        assert_eq!(value, "foo");
    }

    #[test]
    fn path_buf_from_lua() {
        use std::path::PathBuf;
        let value: PathBuf = from_str(r#""foo""#).unwrap();
        assert_eq!(value, PathBuf::from("foo"));
    }

    #[test]
    fn some_option_from_lua() {
        let value: Option<i32> = from_str("42").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn none_option_from_lua() {
        let value: Option<i32> = from_str("nil").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn vec_from_lua() {
        let value: Vec<i32> = from_str("{ 1, 2, 3 }").unwrap();
        assert_eq!(value, vec![1, 2, 3]);
    }

    #[test]
    fn b_tree_set_from_lua() {
        use std::collections::BTreeSet;
        let value: BTreeSet<i32> = from_str("{ 1, 2, 3 }").unwrap();
        assert_eq!(value, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn hash_set_from_lua() {
        use std::collections::HashSet;
        let value: HashSet<i32> = from_str("{ 1, 2, 3 }").unwrap();
        assert_eq!(value, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn b_tree_map_from_lua() {
        use std::collections::BTreeMap;
        let value: BTreeMap<String, i32> = from_str("{ foo = 1, bar = 2, baz = 3 }").unwrap();
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
    fn hash_map_from_lua() {
        use std::collections::HashMap;
        let value: HashMap<String, i32> = from_str("{ foo = 1, bar = 2, baz = 3 }").unwrap();
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
    fn value_from_lua() {
        use crate::util::value::Value;
        let value: Value = from_str(
            r#"{
                bool = true,
                int = 1,
                float = 3.14,
                string = "bar",
                unit = nil,
                vec = { 1, 2, 3 },
                map = { baz = 1 },
            }"#,
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
    fn function_from_lua() {
        use crate::util::function::Function;
        let f: Function<(i32, i32), i32> = from_str("function (x, y) return x + y end").unwrap();
        assert_eq!(f.call(1, 2).unwrap(), 3);
    }

    #[test]
    fn derive_struct_from_lua() {
        #[derive(FromLua)]
        struct Data {
            foo: String,
        }

        let result: Data = from_str(r#"{ foo = "bar" }"#).unwrap();
        assert_eq!(result.foo, "bar");
    }

    #[test]
    fn derive_struct_skip_from_lua() {
        #[derive(FromLua)]
        struct Data {
            #[vitrine(skip)]
            foo: String,
        }

        let result: Data = from_str(r#"{ foo = "bar" }"#).unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_from_lua() {
        #[derive(FromLua)]
        struct Data {
            #[vitrine(default)]
            foo: String,
        }

        let result: Data = from_str("{}").unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_fn_from_lua() {
        #[derive(FromLua)]
        struct Data {
            #[vitrine(default = "default_foo")]
            foo: String,
        }

        fn default_foo() -> String {
            "bar".to_string()
        }

        let result: Data = from_str("{}").unwrap();
        assert_eq!(result.foo, "bar");
    }
}
