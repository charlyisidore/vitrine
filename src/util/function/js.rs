//! JavaScript function handler.

use std::sync::Arc;

use quickjs_runtime::{
    facades::QuickJsRuntimeFacade,
    values::{CachedJsFunctionRef, JsValueFacade},
};

use super::super::from_js::FromJs;

/// JavaScript function handler.
#[derive(Clone)]
pub(crate) struct Function {
    /// JavaScript function.
    function: Arc<CachedJsFunctionRef>,

    /// Pointer to the JavaScript runtime.
    _runtime: Arc<QuickJsRuntimeFacade>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "js::function")
    }
}

/// Generate a `call_N(...)` method for [`Function`].
macro_rules! impl_js_function_call {
    (
        $method_name:ident($($arg_name:ident: $arg_type:tt),*)
    ) => {
        pub(crate) fn $method_name<$($arg_type,)* R>(&self, $($arg_name: &$arg_type),*)
                -> anyhow::Result<R>
            where
                $(
                    $arg_type: serde::Serialize + ?Sized,
                )*
                R: serde::de::DeserializeOwned,
            {
                let args = Vec::from([$(JsValueFacade::from_serializable(&$arg_name).unwrap(),)*]);

                let result = self.function.invoke_function_sync(args)?;

                let result = futures::executor::block_on(result.to_serde_value())?;

                let result = R::deserialize(result)?;

                Ok(result)
            }
        }
    }

impl Function {
    impl_js_function_call!(call_1(a1: A1));

    impl_js_function_call!(call_2(a1: A1, a2: A2));
}

impl FromJs for Function {
    fn from_js(value: JsValueFacade, runtime: Arc<QuickJsRuntimeFacade>) -> anyhow::Result<Self> {
        match value {
            JsValueFacade::JsFunction { cached_function } => Ok(Self {
                function: Arc::new(cached_function),
                _runtime: runtime,
            }),
            _ => panic!("Expected function, received {}", value.get_value_type()),
        }
    }
}
