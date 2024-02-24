//! A general-purpose function wrapper.

use std::sync::{Arc, Mutex};

/// A general-purpose function wrapper.
///
/// This structure can store and call any faillible function having the
/// specified signature.
///
/// # Example
///
/// ```
/// use vitrine::util::function::Function;
///
/// let func: Function<(i32,), i32> = Function::from(|(x,)| Ok(2 * x));
///
/// let result = func.call((1,)).unwrap();
///
/// assert_eq!(result, 2);
/// ```
#[derive(Clone)]
pub struct Function<A, R>(Arc<Mutex<DynFnMut<A, R>>>);

/// Function type inside `Arc<Mutex<...>>`.
type DynFnMut<A, R> = dyn FnMut(A) -> Result<R, FunctionError> + Send + Sync;

/// Error returned by [`Function`].
pub type FunctionError = Box<dyn std::error::Error + Send + Sync>;

impl<A, R> Function<A, R> {
    pub fn call(&self, args: A) -> Result<R, FunctionError> {
        let mut f = self.0.lock().unwrap();
        f(args)
    }
}

impl<A, R> std::fmt::Debug for Function<A, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function")
    }
}

impl<T, A, R> From<T> for Function<A, R>
where
    T: FnMut(A) -> Result<R, FunctionError> + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}
