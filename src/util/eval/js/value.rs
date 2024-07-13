//! Provides [`JsValue`].

use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

/// Represents a JavaScript value.
pub enum JsValue {
    /// Null.
    Null,
    /// Undefined.
    Undefined,
    /// Boolean.
    Boolean(bool),
    /// Number.
    Number(f64),
    /// String.
    String(String),
    /// Array.
    Array(Vec<Self>),
    /// Object.
    Object(HashMap<String, Self>),
    /// Date.
    Date(f64),
    /// Function.
    Function(
        usize,
        Box<dyn Fn(Vec<Self>) -> anyhow::Result<Self> + Send + Sync>,
    ),
}

impl JsValue {
    /// Return the type as a string.
    pub fn type_str(&self) -> &'static str {
        use JsValue::*;
        match self {
            Null => "null",
            Undefined => "undefined",
            Boolean(..) => "boolean",
            Number(..) => "number",
            String(..) => "string",
            Array(..) => "Array",
            Object(..) => "object",
            Date(..) => "Date",
            Function(..) => "function",
        }
    }

    /// Convert a [`v8::Value`] to a [`JsValue`].
    pub fn from_v8<'s>(
        scope: &mut v8::HandleScope<'s>,
        value: v8::Local<'s, v8::Value>,
        sender: &Sender<(usize, Vec<Self>)>,
        receiver: &Arc<Mutex<Receiver<anyhow::Result<Self>>>>,
        functions: &mut Vec<v8::Local<'s, v8::Function>>,
    ) -> Self {
        if value.is_null() {
            Self::Null
        } else if value.is_undefined() {
            Self::Undefined
        } else if value.is_boolean() {
            Self::Boolean(value.is_true())
        } else if value.is_number() {
            Self::Number(value.number_value(scope).unwrap())
        } else if value.is_string() {
            Self::String(value.to_rust_string_lossy(scope))
        } else if value.is_date() {
            Self::Date(v8::Local::<v8::Date>::try_from(value).unwrap().value_of())
        } else if value.is_function() {
            let sender = sender.clone();
            let receiver = receiver.clone();
            let index = functions.len();
            functions.push(value.clone().try_into().unwrap());
            Self::Function(
                index,
                Box::new(move |args: Vec<Self>| {
                    sender.send((index, args)).unwrap();
                    receiver.lock().unwrap().recv().unwrap()
                }),
            )
        } else if value.is_array() {
            let array = v8::Local::<v8::Array>::try_from(value).unwrap();
            Self::Array(
                (0..array.length())
                    .map(|index| {
                        let v = array.get_index(scope, index).unwrap();
                        Self::from_v8(scope, v, sender, receiver, functions)
                    })
                    .collect(),
            )
        } else if value.is_object() {
            let object = v8::Local::<v8::Object>::try_from(value).unwrap();
            let keys = {
                object
                    .get_own_property_names(scope, Default::default())
                    .unwrap()
            };
            let map = (0..keys.length())
                .map(|i| {
                    let k = keys.get_index(scope, i).unwrap();
                    let v = object.get(scope, k).unwrap();
                    let k = k.to_rust_string_lossy(scope);
                    let v = Self::from_v8(scope, v, sender, receiver, functions);
                    (k, v)
                })
                .collect();
            Self::Object(map)
        } else {
            unimplemented!()
        }
    }

    /// Convert a [`JsValue`] to a [`v8::Value`].
    pub fn into_v8<'s>(
        self,
        scope: &mut v8::HandleScope<'s>,
        functions: &Vec<v8::Local<'s, v8::Function>>,
    ) -> v8::Local<'s, v8::Value> {
        use JsValue::*;
        match self {
            Null => v8::null(scope).into(),
            Undefined => v8::undefined(scope).into(),
            Boolean(v) => v8::Boolean::new(scope, v).into(),
            Number(v) => v8::Number::new(scope, v).into(),
            String(v) => v8::String::new(scope, &v).unwrap().into(),
            Array(vec) => {
                let elements: Vec<_> = vec
                    .into_iter()
                    .map(|v| Self::into_v8(v, scope, functions))
                    .collect();
                v8::Array::new_with_elements(scope, elements.as_slice()).into()
            },
            Object(map) => {
                let (keys, values): (Vec<_>, Vec<_>) = map
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            v8::Local::<v8::Name>::from(v8::String::new(scope, &k).unwrap()),
                            Self::into_v8(v, scope, functions),
                        )
                    })
                    .unzip();
                let null = v8::null(scope);
                v8::Object::with_prototype_and_properties(
                    scope,
                    null.into(),
                    keys.as_slice(),
                    values.as_slice(),
                )
                .into()
            },
            Date(v) => v8::Date::new(scope, v).unwrap().into(),
            Function(i, _) => functions.get(i).unwrap().clone().into(),
        }
    }
}

impl std::fmt::Debug for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use JsValue::*;
        match self {
            Null => write!(f, "Null"),
            Undefined => write!(f, "Undefined"),
            Boolean(v) => f.debug_tuple("Boolean").field(v).finish(),
            Number(v) => f.debug_tuple("Number").field(v).finish(),
            String(v) => f.debug_tuple("String").field(v).finish(),
            Array(v) => f.debug_tuple("Array").field(v).finish(),
            Object(v) => f.debug_tuple("Object").field(v).finish(),
            Date(v) => f.debug_tuple("Date").field(v).finish(),
            Function(..) => write!(f, "Function"),
        }
    }
}
