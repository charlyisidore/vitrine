//! Rhai function handler.

use std::sync::Arc;

use super::super::from_rhai::FromRhai;

#[derive(Clone)]
pub(crate) struct Function {
    engine: Arc<rhai::Engine>,
    ast: Arc<rhai::AST>,
    fn_ptr: Arc<rhai::FnPtr>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rhai::function::{:?}", self.fn_ptr)
    }
}

/// Generate a `call_N()` method for [`Function`].
macro_rules! impl_rhai_function_call {
    (
        $(#[$($attrs:tt)*])*
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
            $(
                let $arg_name = rhai::serde::to_dynamic($arg_name)?.to_owned();
            )*

            let result = self
                .fn_ptr
                .call::<rhai::Dynamic>(&self.engine, &self.ast, ($($arg_name,)*))?;

            let result = rhai::serde::from_dynamic(&result)?;

            Ok(result)
        }
    }
}

impl Function {
    impl_rhai_function_call!(call_1(a1: A1));

    impl_rhai_function_call!(call_2(a1: A1, a2: A2));
}

impl FromRhai for Function {
    fn from_rhai(
        value: &rhai::Dynamic,
        engine: Arc<rhai::Engine>,
        ast: Arc<rhai::AST>,
    ) -> anyhow::Result<Self> {
        let fn_ptr =
            Arc::new(value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                anyhow::anyhow!("Expected Function, received {}", value.type_name())
            })?);

        Ok(Function {
            engine,
            ast,
            fn_ptr,
        })
    }
}
