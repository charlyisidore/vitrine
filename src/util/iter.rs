//! Utility methods for iterators.
//!
//! This module is inspired by [`itertools`](https://docs.rs/itertools).

/// Extend [`Iterator`] with utility methods.
pub trait IteratorExt
where
    Self: Iterator,
{
    /// Creates an iterator that applies the given closure to every [`Ok`]
    /// value.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)].into_iter().map_ok(|x| 2 * x);
    ///
    /// assert_eq!(iter.next(), Some(Ok(2)));
    /// assert_eq!(iter.next(), Some(Err(2)));
    /// assert_eq!(iter.next(), Some(Ok(6)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn map_ok<F, T, U, E>(self, f: F) -> MapOk<Self, F>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnMut(T) -> U,
    {
        MapOk { iter: self, f }
    }

    /// Creates an iterator that applies the given closure to every [`Err`]
    /// value.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .map_err(|x| 2 * x);
    ///
    /// assert_eq!(iter.next(), Some(Ok(1)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), Some(Ok(3)));
    /// assert_eq!(iter.next(), Some(Err(8)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn map_err<O, T, E, F>(self, f: O) -> MapErr<Self, O>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        O: FnMut(E) -> F,
    {
        MapErr { iter: self, f }
    }

    /// Creates an iterator that applies the given faillible closure to every
    /// [`Ok`] value.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .map_ok_try(|x| Ok(2 * x));
    ///
    /// assert_eq!(iter.next(), Some(Ok(2)));
    /// assert_eq!(iter.next(), Some(Err(2)));
    /// assert_eq!(iter.next(), Some(Ok(6)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn map_ok_try<F, T, U, E>(self, f: F) -> MapOkTry<Self, F>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnMut(T) -> Result<U, E>,
    {
        MapOkTry { iter: self, f }
    }

    /// Creates an iterator that applies the given faillible closure to every
    /// [`Err`] value.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .map_err_try(|x| Err(2 * x));
    ///
    /// assert_eq!(iter.next(), Some(Ok(1)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), Some(Ok(3)));
    /// assert_eq!(iter.next(), Some(Err(8)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn map_err_try<O, T, E, F>(self, f: O) -> MapErrTry<Self, O>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        O: FnMut(E) -> Result<T, F>,
    {
        MapErrTry { iter: self, f }
    }

    /// Creates an iterator that filters every [`Ok`] value with the given
    /// predicate.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .filter_ok(|x| *x != 1);
    ///
    /// assert_eq!(iter.next(), Some(Err(2)));
    /// assert_eq!(iter.next(), Some(Ok(3)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn filter_ok<P, T, E>(self, predicate: P) -> FilterOk<Self, P>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        P: FnMut(&T) -> bool,
    {
        FilterOk {
            iter: self,
            predicate,
        }
    }

    /// Creates an iterator that filters every [`Err`] value with the given
    /// predicate.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .filter_err(|x| *x != 2);
    ///
    /// assert_eq!(iter.next(), Some(Ok(1)));
    /// assert_eq!(iter.next(), Some(Ok(3)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn filter_err<P, T, E>(self, predicate: P) -> FilterErr<Self, P>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        P: FnMut(&E) -> bool,
    {
        FilterErr {
            iter: self,
            predicate,
        }
    }

    /// Creates an iterator that filters every [`Ok`] value with the given
    /// faillible predicate.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .filter_ok_try(|x| if *x == 1 { Ok(false) } else { Err(2 * x) });
    ///
    /// assert_eq!(iter.next(), Some(Err(2)));
    /// assert_eq!(iter.next(), Some(Err(6)));
    /// assert_eq!(iter.next(), Some(Err(4)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn filter_ok_try<P, T, E>(self, predicate: P) -> FilterOkTry<Self, P>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        P: FnMut(&T) -> Result<bool, E>,
    {
        FilterOkTry {
            iter: self,
            predicate,
        }
    }

    /// Creates an iterator that filters every [`Err`] value with the given
    /// faillible predicate.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let mut iter = [Ok(1), Err(2), Ok(3), Err(4)]
    ///     .into_iter()
    ///     .filter_err_try(|x| if *x == 2 { Ok(false) } else { Err(2 * x) });
    ///
    /// assert_eq!(iter.next(), Some(Ok(1)));
    /// assert_eq!(iter.next(), Some(Ok(3)));
    /// assert_eq!(iter.next(), Some(Err(8)));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn filter_err_try<P, T, E>(self, predicate: P) -> FilterErrTry<Self, P>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        P: FnMut(&E) -> Result<bool, E>,
    {
        FilterErrTry {
            iter: self,
            predicate,
        }
    }

    /// Transforms an iterator of faillible items into a faillible collection.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let res = ["1", "2", "3"]
    ///     .into_iter()
    ///     .map(|s| s.parse::<i32>())
    ///     .try_collect();
    ///
    /// assert_eq!(res, Ok(vec![1, 2, 3]));
    /// ```
    fn try_collect<T, U, E>(self) -> Result<U, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        Result<U, E>: FromIterator<Result<T, E>>,
    {
        self.collect()
    }

    /// Apply a function to an adapter of the current iterator.
    ///
    /// The adapter iterates over [`Ok`] values until the first [`Err`] value is
    /// met.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let res = ["1", "a", "3"]
    ///     .into_iter()
    ///     .map(|s| s.parse::<i32>())
    ///     .process_results(|iter| iter.min());
    ///
    /// assert!(res.is_err());
    /// ```
    fn process_results<F, T, E, R>(self, f: F) -> Result<R, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnOnce(ProcessResults<Self, E>) -> R,
    {
        let mut error = None;
        let result = f(ProcessResults {
            iter: self,
            error: &mut error,
        });
        error.map_or_else(|| Ok(result), |e| Err(e))
    }

    /// Apply a faillible function to an adapter of the current iterator.
    ///
    /// The adapter iterates over [`Ok`] values until the first [`Err`] value is
    /// met.
    ///
    /// ```
    /// use vitrine::util::iter::IteratorExt;
    ///
    /// let res = ["1", "a", "3"]
    ///     .into_iter()
    ///     .map(|s| s.parse::<i32>())
    ///     .process_results_try(|iter| Ok(iter.min()));
    ///
    /// assert!(res.is_err());
    /// ```
    fn process_results_try<F, T, E, R>(self, f: F) -> Result<R, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnOnce(ProcessResults<Self, E>) -> Result<R, E>,
    {
        let mut error = None;
        let result = f(ProcessResults {
            iter: self,
            error: &mut error,
        })?;
        error.map_or_else(|| Ok(result), |e| Err(e))
    }
}

impl<T> IteratorExt for T where T: Iterator {}

/// An iterator that applies a function to every [`Ok`] value.
#[derive(Clone, Debug)]
pub struct MapOk<I, F> {
    iter: I,
    f: F,
}

impl<I, F, T, U, E> Iterator for MapOk<I, F>
where
    I: Iterator<Item = Result<T, E>>,
    F: FnMut(T) -> U,
{
    type Item = Result<U, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|r| r.map(|v| (self.f)(v)))
    }
}

/// An iterator that applies a function to every [`Err`] value.
#[derive(Clone, Debug)]
pub struct MapErr<I, O> {
    iter: I,
    f: O,
}

impl<I, O, T, E, F> Iterator for MapErr<I, O>
where
    I: Iterator<Item = Result<T, E>>,
    O: FnMut(E) -> F,
{
    type Item = Result<T, F>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|r| r.map_err(|e| (self.f)(e)))
    }
}

/// An iterator that applies a faillible function to every [`Ok`] value.
#[derive(Clone, Debug)]
pub struct MapOkTry<I, F> {
    iter: I,
    f: F,
}

impl<I, F, T, U, E> Iterator for MapOkTry<I, F>
where
    I: Iterator<Item = Result<T, E>>,
    F: FnMut(T) -> Result<U, E>,
{
    type Item = Result<U, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|r| r.and_then(|v| (self.f)(v)))
    }
}

/// An iterator that applies a faillible function to every [`Err`] value.
#[derive(Clone, Debug)]
pub struct MapErrTry<I, F> {
    iter: I,
    f: F,
}

impl<I, O, T, E, F> Iterator for MapErrTry<I, O>
where
    I: Iterator<Item = Result<T, E>>,
    O: FnMut(E) -> Result<T, F>,
{
    type Item = Result<T, F>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|r| r.or_else(|e| (self.f)(e)))
    }
}

/// An iterator that filters every [`Ok`] value with a predicate.
#[derive(Clone, Debug)]
pub struct FilterOk<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P, T, E> Iterator for FilterOk<I, P>
where
    I: Iterator<Item = Result<T, E>>,
    P: FnMut(&T) -> bool,
{
    type Item = Result<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.iter.next() {
                Some(Ok(v)) => match (self.predicate)(&v) {
                    true => Some(Ok(v)),
                    false => continue,
                },
                v => v,
            };
        }
    }
}

/// An iterator that filters every [`Err`] value with a predicate.
#[derive(Clone, Debug)]
pub struct FilterErr<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P, T, E> Iterator for FilterErr<I, P>
where
    I: Iterator<Item = Result<T, E>>,
    P: FnMut(&E) -> bool,
{
    type Item = Result<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.iter.next() {
                Some(Err(e)) => match (self.predicate)(&e) {
                    true => Some(Err(e)),
                    false => continue,
                },
                v => v,
            };
        }
    }
}

/// An iterator that filters every [`Ok`] value with a faillible predicate.
#[derive(Clone, Debug)]
pub struct FilterOkTry<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P, T, E> Iterator for FilterOkTry<I, P>
where
    I: Iterator<Item = Result<T, E>>,
    P: FnMut(&T) -> Result<bool, E>,
{
    type Item = Result<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.iter.next() {
                Some(Ok(v)) => match (self.predicate)(&v) {
                    Ok(true) => Some(Ok(v)),
                    Ok(false) => continue,
                    Err(e) => Some(Err(e)),
                },
                v => v,
            };
        }
    }
}

/// An iterator that filters every [`Err`] value with a faillible predicate.
#[derive(Clone, Debug)]
pub struct FilterErrTry<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P, T, E> Iterator for FilterErrTry<I, P>
where
    I: Iterator<Item = Result<T, E>>,
    P: FnMut(&E) -> Result<bool, E>,
{
    type Item = Result<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.iter.next() {
                Some(Err(e)) => match (self.predicate)(&e) {
                    Ok(true) => Some(Err(e)),
                    Ok(false) => continue,
                    Err(e) => Some(Err(e)),
                },
                v => v,
            };
        }
    }
}

/// An iterator that iterates over [`Ok`] values until the first [`Err`] value
/// is met.
#[derive(Debug)]
pub struct ProcessResults<'e, I, E: 'e> {
    iter: I,
    error: &'e mut Option<E>,
}

impl<'e, I, T, E> Iterator for ProcessResults<'e, I, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(Ok(v)) => Some(v),
            Some(Err(e)) => {
                *self.error = Some(e);
                None
            },
            None => None,
        }
    }
}
