//! A general-purpose function wrapper.

use std::sync::Arc;

/// A general-purpose function wrapper.
///
/// This structure can store and call any faillible function having the
/// specified signature. It is both [`Clone`] and [`Debug`].
///
/// # Example
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use vitrine::util::function::Function;
///
/// let add: Function<(i32, i32), i32> = Function::from(|x, y| x + y);
/// let result = add.call(1, 2)?;
/// assert_eq!(result, 3);
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Function<A, R>(Arc<dyn Fn(A) -> Result<R, AnyError> + Send + Sync>);

/// Type for storing any type of error.
///
/// This type is used as the error returned by [`Function::call`].
pub type AnyError = anyhow::Error;

macro_rules! impl_function {
    ($($arg:ident: $ty:ident),*) => {
        impl <$($ty,)* R> Function <($($ty,)*), R>
        {
            /// Perform the call operation on the contained function.
            ///
            /// In case of failure, it returns a [`AnyError`] that wraps the error.
            pub fn call(&self, $($arg: $ty),*) -> Result<R, AnyError> {
                (self.0)(($(Into::into($arg),)*))
            }
        }

        impl <$($ty,)* R, E, F> From<F> for Function <($($ty,)*), R>
        where
            F: Fn($($ty),*) -> E + Send + Sync + 'static,
            E: IntoResult<R, AnyError>,
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
                    std::any::type_name::<R>(),
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

/// A trait for converting types to [`Result`].
///
/// This trait is used in [`Function::from`] to accept closures that return
/// arbitrary types.
pub trait IntoResult<T, E> {
    /// Convert this type to a [`Result`] for [`Function`].
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

        let forty_two: Function<(), ()> = Function::from(move || {
            let mut result = result_mutex.lock().unwrap();
            *result = 42;
        });

        let _ = forty_two.call().unwrap();
        let result = result.lock().unwrap();
        assert_eq!(*result, 42);
    }

    #[test]
    fn zero_arg() {
        let forty_two: Function<(), i32> = Function::from(|| 42);
        let result = forty_two.call().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn one_arg() {
        let double: Function<(i32,), i32> = Function::from(|x| 2 * x);
        let result = double.call(1).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn two_args() {
        let add: Function<(i32, i32), i32> = Function::from(|x, y| x + y);
        let result = add.call(1, 2).unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn into_error() {
        let parse: Function<(&str,), i32> = Function::from(|x: &str| x.parse());
        let result: i32 = parse.call("42").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn into_function() {
        fn exec(f: impl Into<Function<(i32,), i32>>) -> i32 {
            f.into().call(42).unwrap()
        }
        let result = exec(|x| 2 * x);
        assert_eq!(result, 2 * 42);
    }

    #[test]
    fn debug() {
        let add: Function<(i32, i32), i32> = Function::from(|x, y| x + y);
        let result = format!("{:?}", add);
        assert_eq!(result, "Function(i32, i32) -> i32");
    }
}
