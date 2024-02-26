//! A general-purpose function wrapper.

use std::sync::Arc;

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
pub struct Function<A, R>(Arc<dyn Fn(A) -> Result<R, BoxedError> + Send + Sync>);

macro_rules! impl_function {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> Function <($($ty,)*), R>
        {
            /// Perform the call operation on the contained function.
            ///
            /// In case of failure, it returns a [`BoxedError`] that wraps the error.
            pub fn call(&self, $($arg: impl Into<$ty>),*) -> Result<R, BoxedError> {
                (self.0)(($(Into::into($arg),)*))
            }
        }

        impl <$($ty,)* R, F, T> From<F> for Function <($($ty,)*), R>
        where
            F: Fn($($ty),*) -> T + Send + Sync + 'static,
            T: IntoResult<R>,
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
/// This structure wraps any error returned by [`Function::call`]. It implements
/// [`std::error::Error`], so it can be easily used with third party libraries.
pub struct BoxedError(Box<dyn std::error::Error + Send + Sync>);

impl BoxedError {
    /// Wrap an error with [`BoxedError`].
    pub fn new(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(e))
    }
}

impl std::fmt::Debug for BoxedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for BoxedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BoxedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for BoxedError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(e)
    }
}

impl From<StringError> for BoxedError {
    fn from(e: StringError) -> Self {
        Self::new(e)
    }
}

/// Helper error type that can be created from a string.
///
/// It implements [`std::error::Error`], so it can be easily used with third
/// party libraries.
pub struct StringError(String);

impl StringError {
    /// Create an error from a string.
    pub fn new(e: impl Into<String>) -> Self {
        Self(Into::into(e))
    }
}

impl std::fmt::Debug for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for StringError {}

impl<T> From<T> for StringError
where
    T: Into<String>,
{
    fn from(e: T) -> Self {
        Self::new(e)
    }
}

/// A trait for converting types to [`Result`] for [`Function`].
///
/// This trait is used in [`Function::from`] to accept closures that return
/// arbitrary types.
pub trait IntoResult<T> {
    /// Convert this type to a [`Result`] for [`Function`].
    fn into_result(self) -> Result<T, BoxedError>;
}

impl<T, E> IntoResult<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_result(self) -> Result<T, BoxedError> {
        self.map_err(BoxedError::new)
    }
}

impl<T> IntoResult<T> for T {
    fn into_result(self) -> Result<T, BoxedError> {
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
