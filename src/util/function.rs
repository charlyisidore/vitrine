//! A general-purpose function wrapper.

use std::{error::Error, sync::Arc};

/// A general-purpose function wrapper.
///
/// This structure can store and call any faillible or infaillible function
/// having the specified signature. It is both [`Clone`] and [`Debug`].
///
/// # Example
///
/// ```
/// use vitrine::util::function::Function;
///
/// let add = Function::from(|x: i32, y: i32| x + y);
///
/// let result = add.call(1, 2).unwrap();
///
/// assert_eq!(result, 3);
/// ```
#[derive(Clone)]
pub struct Function<A, R>(Arc<dyn Fn(A) -> Result<R, FunctionError> + Send + Sync>);

macro_rules! impl_function {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> Function <($($ty,)*), R>
        {
            /// Perform the call operation on the contained function.
            ///
            /// In case of failure, it returns a [`FunctionError`] that wraps the error.
            pub fn call(&self, $($arg: impl Into<$ty>),*) -> Result<R, FunctionError> {
                (self.0)(($(Into::into($arg),)*))
            }
        }

        impl <$($ty,)* R, F, T> From<F> for Function <($($ty,)*), R>
        where
            F: Fn($($ty),*) -> T + Send + Sync + 'static,
            T: IntoResult<R, FunctionError>,
        {
            fn from(f: F) -> Self {
                Self(Arc::new(move |($($arg,)*)| (f)($($arg),*).into_result()))
            }
        }

        impl <$($ty,)* R> std::fmt::Debug for Function <($($ty,)*), R> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "Function{} -> {}",
                    std::any::type_name::<($($ty,)*)>(),
                    std::any::type_name::<R>()
                )
            }
        }
    }
}

impl_function! {}
impl_function! { a1: A1 }
impl_function! { a1: A1, a2: A2 }
impl_function! { a1: A1, a2: A2, a3: A3 }
impl_function! { a1: A1, a2: A2, a3: A3, a4: A4 }
impl_function! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5 }
impl_function! { a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6 }

/// Error type returned by [`Function::call`].
///
/// This structure wraps any error returned by the function contained in
/// [`Function`].
#[derive(Debug)]
pub struct FunctionError(Box<dyn Error>);

impl FunctionError {
    /// Return a reference to the error contained in the structure.
    pub fn inner(&self) -> &dyn Error {
        self.0.as_ref()
    }
}

impl<E> From<E> for FunctionError
where
    E: Error + 'static,
{
    fn from(e: E) -> Self {
        Self(Box::new(e))
    }
}

impl std::fmt::Display for FunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A trait for converting types to [`Result`].
///
/// This trait is used in [`Function::from`] to accept closures that return
/// arbitrary types.
pub trait IntoResult<T, E> {
    /// Convert this type to a [`Result`].
    fn into_result(self) -> Result<T, E>;
}

impl<T, E, F> IntoResult<T, E> for Result<T, F>
where
    F: Into<E>,
{
    fn into_result(self) -> Result<T, E> {
        self.map_err(Into::into)
    }
}

impl<T, E> IntoResult<T, E> for T {
    fn into_result(self) -> Result<T, E> {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Function;

    #[test]
    fn no_return() {
        use std::sync::{Arc, Mutex};

        let result = Arc::new(Mutex::new(0));

        let result_mutex = Arc::clone(&result);
        let forty_two = Function::from(move || {
            let mut result = result_mutex.lock().unwrap();
            *result = 42;
        });

        let _ = forty_two.call().unwrap();
        let result = result.lock().unwrap();
        assert_eq!(*result, 42);
    }

    #[test]
    fn zero_arg() {
        let forty_two = Function::from(|| 42);
        let result = forty_two.call().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn one_arg() {
        let double = Function::from(|x: i32| 2 * x);
        let result = double.call(1).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn two_args() {
        let add = Function::from(|x: i32, y: i32| x + y);
        let result = add.call(1, 2).unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn into_arg() {
        let parse = Function::from(|x: String| x.to_uppercase());
        let result = parse.call("foo").unwrap();
        assert_eq!(result, "FOO");
    }

    #[test]
    fn into_error() {
        let parse = Function::from(|x: &str| x.parse());
        let result: i32 = parse.call("42").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn into_function() {
        fn exec(f: impl Into<Function<(i32,), i32>>) -> i32 {
            f.into().call(42).unwrap()
        }
        let result = exec(|x: i32| 2 * x);
        assert_eq!(result, 2 * 42);
    }

    #[test]
    fn debug() {
        let add = Function::from(|x: i32, y: i32| x + y);
        let result = format!("{:?}", add);
        assert_eq!(result, "Function(i32, i32) -> i32");
    }
}
