//! Read values from JavaScript modules.

pub mod from_js;
pub mod into_js;
pub mod value;

use std::{
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex, Once},
};

pub use from_js::FromJs;
pub use into_js::IntoJs;
use thiserror::Error;
pub use value::JsValue;

static INITIALIZE_V8: Once = Once::new();

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum JsError {
    /// Anyhow error.
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    /// Error converting values from JavaScript.
    #[error("{}", format_from_js_error(.from, .to, .message))]
    FromJs {
        /// Source type name.
        from: &'static str,
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
    /// V8 data error.
    #[error(transparent)]
    V8DataError(#[from] v8::DataError),
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

    Ok(from_str(source)?)
}

/// Read value from a JavaScript script string.
pub fn from_str<T>(source: String) -> Result<T, JsError>
where
    T: FromJs,
{
    let (sender, receiver) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || run(&source, sender));

    let value = receiver
        .recv()
        .map_err(|_| handle.join().unwrap().err().unwrap())?;

    T::from_js(value)
}

/// Initialize the [`v8`] engine.
fn initialize_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

/// Run a JavaScript code using the [`v8`] engine.
fn run(code: &str, value_sender: Sender<JsValue>) -> anyhow::Result<()> {
    INITIALIZE_V8.call_once(initialize_v8);

    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let code = v8::String::new(scope, &code).unwrap();

    let resource_name = v8::String::new(scope, "index.js").unwrap();
    let source_map_url = v8::undefined(scope);
    let origin = v8::ScriptOrigin::new(
        scope,
        resource_name.into(),
        0,
        0,
        false,
        0,
        source_map_url.into(),
        false,
        false,
        true,
    );

    let source = v8::script_compiler::Source::new(code, Some(&origin));

    let module = with_try_catch(scope, |scope| {
        v8::script_compiler::compile_module(scope, source)
    })?;

    module
        .instantiate_module(scope, |_, _, _, _| unimplemented!())
        .unwrap();

    module.evaluate(scope).unwrap();

    if module.get_status() == v8::ModuleStatus::Errored {
        return Err(anyhow::anyhow!(module
            .get_exception()
            .to_rust_string_lossy(scope)));
    }

    let module_namespace = module.get_module_namespace();
    let object = module_namespace.to_object(scope).unwrap();

    let key = v8::String::new(scope, "default").unwrap();
    let module_value = object.get(scope, key.into()).unwrap();

    let (call_sender, call_receiver) = std::sync::mpsc::channel::<(usize, Vec<JsValue>)>();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    let result_receiver = Arc::new(Mutex::new(result_receiver));

    let mut functions = Vec::<v8::Local<v8::Function>>::new();

    let value = JsValue::from_v8(
        scope,
        module_value,
        &call_sender,
        &result_receiver,
        &mut functions,
    );
    value_sender.send(value).unwrap();

    while let Ok((index, args)) = call_receiver.recv() {
        let function = functions.get(index).unwrap();
        let this = v8::undefined(scope).into();
        let args: Vec<_> = args
            .into_iter()
            .map(|value| value.into_v8(scope, &functions))
            .collect();

        let result = with_try_catch(scope, |scope| function.call(scope, this, args.as_slice()))?;

        let result = JsValue::from_v8(
            scope,
            result,
            &call_sender,
            &result_receiver,
            &mut functions,
        );

        result_sender.send(Ok(result)).unwrap();
    }

    Ok(())
}

/// Wrap a closure inside a [`v8::TryCatch`] scope.
fn with_try_catch<'s, R>(
    scope: &mut v8::HandleScope<'s>,
    f: impl FnOnce(&mut v8::HandleScope<'s>) -> Option<R>,
) -> anyhow::Result<R> {
    let mut tc_scope = v8::TryCatch::new(scope);
    match f(&mut tc_scope) {
        Some(result) => Ok(result),
        None => {
            debug_assert!(tc_scope.has_caught());
            let exception = tc_scope.exception().unwrap();
            drop(tc_scope);
            Err(anyhow::anyhow!(exception.to_rust_string_lossy(scope)))
        },
    }
}

/// Format [`JsError::FromJs`] error.
///
/// Add parentheses around `message` if not empty.
fn format_from_js_error(from: &str, to: &str, message: &Option<String>) -> String {
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

    use super::{FromJs, JsError};

    fn from_str<T>(source: &str) -> Result<T, JsError>
    where
        T: FromJs,
    {
        super::from_str(source.to_string())
    }

    #[test]
    fn bool_from_js() {
        let value: bool = from_str("export default true;").unwrap();
        assert_eq!(value, true);
    }

    #[test]
    fn f64_from_js() {
        let value: f64 = from_str("export default 3.14;").unwrap();
        assert_eq!(value, 3.14);
    }

    #[test]
    fn i32_from_js() {
        let value: i32 = from_str("export default -42;").unwrap();
        assert_eq!(value, -42);
    }

    #[test]
    fn u32_from_js() {
        let value: u32 = from_str("export default 42;").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn string_from_js() {
        let value: String = from_str(r#"export default "foo";"#).unwrap();
        assert_eq!(value, "foo");
    }

    #[test]
    fn path_buf_from_js() {
        use std::path::PathBuf;
        let value: PathBuf = from_str(r#"export default "foo";"#).unwrap();
        assert_eq!(value, PathBuf::from("foo"));
    }

    #[test]
    fn url_buf_from_js() {
        use crate::util::url::Url;
        let value: Url = from_str(r#"export default "foo";"#).unwrap();
        assert_eq!(value, Url::from("foo"));
    }

    #[test]
    fn some_option_from_js() {
        let value: Option<i32> = from_str("export default 42;").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn none_option_from_js_null() {
        let value: Option<i32> = from_str("export default null;").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn none_option_from_js_undefined() {
        let value: Option<i32> = from_str("export default undefined;").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn vec_from_js() {
        let value: Vec<i32> = from_str("export default [1, 2, 3];").unwrap();
        assert_eq!(value, vec![1, 2, 3]);
    }

    #[test]
    fn b_tree_set_from_js() {
        use std::collections::BTreeSet;
        let value: BTreeSet<i32> = from_str("export default [1, 2, 3];").unwrap();
        assert_eq!(value, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn hash_set_from_js() {
        use std::collections::HashSet;
        let value: HashSet<i32> = from_str("export default [1, 2, 3];").unwrap();
        assert_eq!(value, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn b_tree_map_from_js() {
        use std::collections::BTreeMap;
        let value: BTreeMap<String, i32> =
            from_str("export default { foo: 1, bar: 2, baz: 3 };").unwrap();
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
        let value: HashMap<String, i32> =
            from_str("export default { foo: 1, bar: 2, baz: 3 };").unwrap();
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
            r#"export default {
                bool: true,
                int: 1,
                float: 3.14,
                string: "bar",
                unit: null,
                vec: [1, 2, 3],
                map: { baz: 1 },
            }"#,
        )
        .unwrap();
        assert_eq!(
            value,
            Value::Map(
                [
                    ("bool".into(), Value::Bool(true)),
                    ("int".into(), Value::F64(1.0)),
                    ("float".into(), Value::F64(3.14)),
                    ("string".into(), Value::Str("bar".into())),
                    ("unit".into(), Value::Unit),
                    (
                        "vec".into(),
                        Value::Seq([Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)].into()),
                    ),
                    (
                        "map".into(),
                        Value::Map([("baz".into(), Value::F64(1.0))].into()),
                    ),
                ]
                .into(),
            )
        );
    }

    #[test]
    fn function_from_js() {
        use crate::util::function::Function;
        let f: Function<(i32, i32), i32> = from_str("export default (x, y) => x + y;").unwrap();
        assert_eq!(f.call(1, 2).unwrap(), 3);
    }

    #[test]
    fn derive_struct_from_js() {
        #[derive(FromJs)]
        struct Data {
            foo: String,
        }

        let result: Data = from_str(r#"export default { foo: "bar" };"#).unwrap();
        assert_eq!(result.foo, "bar");
    }

    #[test]
    fn derive_struct_skip_from_js() {
        #[derive(FromJs)]
        struct Data {
            #[vitrine(skip)]
            foo: String,
        }

        let result: Data = from_str(r#"export default { foo: "bar" };"#).unwrap();
        assert_eq!(result.foo, "");
    }

    #[test]
    fn derive_struct_default_from_js() {
        #[derive(FromJs)]
        struct Data {
            #[vitrine(default)]
            foo: String,
        }

        let result: Data = from_str("export default {};").unwrap();
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

        let result: Data = from_str("export default {};").unwrap();
        assert_eq!(result.foo, "bar");
    }
}
