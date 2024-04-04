//! Execute pipelines.

use std::sync::mpsc::channel;
pub use std::sync::mpsc::{Receiver, Sender};

/// A pipeline.
///
/// # Example
///
/// ```
/// use std::collections::BTreeSet;
///
/// use vitrine::util::pipeline::Pipeline;
///
/// let (p1, p2) = Pipeline::from_iter(["1", "two", "2", "NaN", "4", "four", "5"])
///     .pipe(|rx, tx| {
///         for s in rx {
///             if let Ok(t) = s.parse() {
///                 tx.send(t).unwrap();
///             }
///         }
///     })
///     .fork(|rx, (tx_even, tx_odd)| {
///         for t in rx {
///             if t % 2 == 0 {
///                 tx_even.send(t).unwrap();
///             } else {
///                 tx_odd.send(t).unwrap();
///             }
///         }
///     });
///
/// let p1 = p1.map(|t| t / 2);
///
/// let result: BTreeSet<i32> = Pipeline::merge((p1, p2), |(rx1, rx2), tx| {
///     for t in rx1.into_iter().chain(rx2) {
///         tx.send(t).unwrap();
///     }
/// })
/// .into_iter()
/// .collect();
///
/// assert_eq!(BTreeSet::from([1, 2, 5]), result);
/// ```
#[derive(Debug)]
pub struct Pipeline<T>(Receiver<T>);

impl<T> Pipeline<T> {
    /// Create a pipeline from a closure.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::new(|tx| {
    ///     for t in 1..=3 {
    ///         tx.send(t).unwrap();
    ///     }
    /// })
    /// .into_iter()
    /// .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2, 3]), result);
    /// ```
    pub fn new(f: impl FnOnce(Sender<T>)) -> Self {
        let (tx, rx) = channel();
        (f)(tx);
        Self(rx)
    }

    /// Create a pipeline from a fallible closure.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{collections::BTreeSet, sync::mpsc::SendError};
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::new_try(|tx| {
    ///     for t in 1..=3 {
    ///         tx.send(t)?;
    ///     }
    ///     Ok::<(), SendError<i32>>(())
    /// })
    /// .unwrap()
    /// .into_iter()
    /// .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2, 3]), result);
    /// ```
    pub fn new_try<E>(f: impl FnOnce(Sender<T>) -> Result<(), E>) -> Result<Self, E> {
        let (tx, rx) = channel();
        (f)(tx)?;
        Ok(Self(rx))
    }

    /// Call a closure that takes one receiver and one sender.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([1, 2, 3])
    ///     .pipe(|rx, tx| {
    ///         for t in rx {
    ///             tx.send(2 * t).unwrap();
    ///         }
    ///     })
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([2, 4, 6]), result);
    /// ```
    pub fn pipe<U>(self, f: impl FnOnce(Receiver<T>, Sender<U>)) -> Pipeline<U> {
        let (tx, rx) = channel();
        (f)(self.0, tx);
        Pipeline(rx)
    }

    /// Call a fallible closure that takes one receiver and one sender.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{collections::BTreeSet, sync::mpsc::SendError};
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([1, 2, 3])
    ///     .pipe_try(|rx, tx| {
    ///         for t in rx {
    ///             tx.send(2 * t)?;
    ///         }
    ///         Ok::<(), SendError<i32>>(())
    ///     })
    ///     .unwrap()
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([2, 4, 6]), result);
    /// ```
    pub fn pipe_try<E, U>(
        self,
        f: impl FnOnce(Receiver<T>, Sender<U>) -> Result<(), E>,
    ) -> Result<Pipeline<U>, E> {
        let (tx, rx) = channel();
        (f)(self.0, tx)?;
        Ok(Pipeline(rx))
    }

    /// Call a closure that merges multiple pipelines into one pipeline.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let p1 = Pipeline::from_iter([1, 2, 3]);
    /// let p2 = Pipeline::from_iter([4, 5, 6]);
    ///
    /// let result: BTreeSet<i32> = Pipeline::merge((p1, p2), |(rx1, rx2), tx| {
    ///     for t in rx1.iter().chain(rx2.iter()) {
    ///         tx.send(t).unwrap();
    ///     }
    /// })
    /// .into_iter()
    /// .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2, 3, 4, 5, 6]), result);
    /// ```
    pub fn merge<P>(p: P, f: impl FnOnce(P::ReceiverTuple, Sender<T>)) -> Self
    where
        P: Merge,
    {
        P::merge(p, f)
    }

    /// Call a fallible closure that merges multiple pipelines into one
    /// pipeline.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{collections::BTreeSet, sync::mpsc::SendError};
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let p1 = Pipeline::from_iter([1, 2, 3]);
    /// let p2 = Pipeline::from_iter([4, 5, 6]);
    ///
    /// let result: BTreeSet<i32> = Pipeline::merge_try((p1, p2), |(rx1, rx2), tx| {
    ///     for t in rx1.iter().chain(rx2.iter()) {
    ///         tx.send(t)?;
    ///     }
    ///     Ok::<(), SendError<i32>>(())
    /// })
    /// .unwrap()
    /// .into_iter()
    /// .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2, 3, 4, 5, 6]), result);
    /// ```
    pub fn merge_try<E, P>(
        p: P,
        f: impl FnOnce(P::ReceiverTuple, Sender<T>) -> Result<(), E>,
    ) -> Result<Self, E>
    where
        P: MergeTry,
    {
        P::merge_try(p, f)
    }

    /// Call a closure that forks a pipeline into multiple pipelines.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let (p1, p2) = Pipeline::from_iter([1, 2, 3]).fork(|rx, (tx_even, tx_odd)| {
    ///     for t in rx {
    ///         if t % 2 == 0 {
    ///             tx_even.send(t).unwrap();
    ///         } else {
    ///             tx_odd.send(t).unwrap();
    ///         }
    ///     }
    /// });
    ///
    /// let evens: BTreeSet<i32> = p1.into_iter().collect();
    /// let odds: BTreeSet<i32> = p2.into_iter().collect();
    ///
    /// assert_eq!(BTreeSet::from([2]), evens);
    /// assert_eq!(BTreeSet::from([1, 3]), odds);
    /// ```
    pub fn fork<P>(self, f: impl FnOnce(Receiver<T>, P)) -> P::PipelineTuple
    where
        P: Fork,
    {
        P::fork(self, f)
    }

    /// Call a fallible closure that forks a pipeline into multiple pipelines.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{collections::BTreeSet, sync::mpsc::SendError};
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let (p1, p2) = Pipeline::from_iter([1, 2, 3])
    ///     .fork_try(|rx, (tx_even, tx_odd)| {
    ///         for t in rx {
    ///             if t % 2 == 0 {
    ///                 tx_even.send(t)?;
    ///             } else {
    ///                 tx_odd.send(t)?;
    ///             }
    ///         }
    ///         Ok::<(), SendError<i32>>(())
    ///     })
    ///     .unwrap();
    ///
    /// let evens: BTreeSet<i32> = p1.into_iter().collect();
    /// let odds: BTreeSet<i32> = p2.into_iter().collect();
    ///
    /// assert_eq!(BTreeSet::from([2]), evens);
    /// assert_eq!(BTreeSet::from([1, 3]), odds);
    /// ```
    pub fn fork_try<E, P>(
        self,
        f: impl FnOnce(Receiver<T>, P) -> Result<(), E>,
    ) -> Result<P::PipelineTuple, E>
    where
        P: ForkTry,
    {
        P::fork_try(self, f)
    }

    /// Call a closure on each element of the pipeline.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([1, 2, 3])
    ///     .map(|x| 2 * x)
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([2, 4, 6]), result);
    /// ```
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Pipeline<U> {
        self.pipe(|rx, tx| {
            for t in rx {
                tx.send((f)(t)).unwrap();
            }
        })
    }

    /// Call a fallible closure on each element of the pipeline.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([1, 2, 3])
    ///     .map_try(|x| Ok::<_, ()>(2 * x))
    ///     .unwrap()
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([2, 4, 6]), result);
    /// ```
    pub fn map_try<E, U>(self, mut f: impl FnMut(T) -> Result<U, E>) -> Result<Pipeline<U>, E> {
        self.pipe_try(|rx, tx| {
            for t in rx {
                tx.send((f)(t)?).unwrap();
            }
            Ok(())
        })
    }

    /// Call a closure to determine if an element should be sent to the next
    /// stage.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([0i32, 1, 2])
    ///     .filter(|x| x.is_positive())
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2]), result);
    /// ```
    pub fn filter(self, mut f: impl FnMut(&T) -> bool) -> Self {
        self.pipe(|rx, tx| {
            for t in rx {
                if (f)(&t) {
                    tx.send(t).unwrap();
                }
            }
        })
    }

    /// Call a fallible closure to determine if an element should be sent to the
    /// next stage.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter([0i32, 1, 2])
    ///     .filter_try(|x| Ok::<_, ()>(x.is_positive()))
    ///     .unwrap()
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 2]), result);
    /// ```
    pub fn filter_try<E>(self, mut f: impl FnMut(&T) -> Result<bool, E>) -> Result<Self, E> {
        self.pipe_try(|rx, tx| {
            for t in rx {
                if (f)(&t)? {
                    tx.send(t).unwrap();
                }
            }
            Ok(())
        })
    }

    /// Call a closure to both filter and map elements.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter(["1", "two", "NaN", "four", "5"])
    ///     .filter_map(|s| s.parse().ok())
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 5]), result);
    /// ```
    pub fn filter_map<U>(self, mut f: impl FnMut(T) -> Option<U>) -> Pipeline<U> {
        self.pipe(|rx, tx| {
            for t in rx {
                if let Some(u) = (f)(t) {
                    tx.send(u).unwrap();
                }
            }
        })
    }

    /// Call a fallible closure to both filter and map elements.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use vitrine::util::pipeline::Pipeline;
    ///
    /// let result: BTreeSet<i32> = Pipeline::from_iter(["1", "two", "NaN", "four", "5"])
    ///     .filter_map_try(|s| Ok::<_, ()>(s.parse().ok()))
    ///     .unwrap()
    ///     .into_iter()
    ///     .collect();
    ///
    /// assert_eq!(BTreeSet::from([1, 5]), result);
    /// ```
    pub fn filter_map_try<E, U>(
        self,
        mut f: impl FnMut(T) -> Result<Option<U>, E>,
    ) -> Result<Pipeline<U>, E> {
        self.pipe_try(|rx, tx| {
            for t in rx {
                if let Some(u) = (f)(t)? {
                    tx.send(u).unwrap();
                }
            }
            Ok(())
        })
    }
}

impl<T> FromIterator<T> for Pipeline<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(|tx| {
            for t in iter {
                tx.send(t).unwrap();
            }
        })
    }
}

impl<T> IntoIterator for Pipeline<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0)
    }
}

/// An iterator over the elements of a [`Pipeline`].
#[derive(Debug)]
pub struct IntoIter<T>(Receiver<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.recv().ok()
    }
}

/// A trait to merge multiple pipelines into one pipeline.
pub trait Merge {
    /// Types of element receivers.
    type ReceiverTuple;

    /// Merge multiple pipelines into one pipeline.
    fn merge<T>(p: Self, f: impl FnOnce(Self::ReceiverTuple, Sender<T>)) -> Pipeline<T>;
}

/// Implements [`Merge`].
macro_rules! impl_merge {
    ($($t:ident $p:ident),*) => {
        impl <$($t),*> Merge for ($(Pipeline<$t>,)*) {
            type ReceiverTuple = ($(Receiver<$t>,)*);

            fn merge<T>(
                p: Self,
                f: impl FnOnce(Self::ReceiverTuple, Sender<T>),
            ) -> Pipeline<T> {
                let ($($p,)*) = p;
                let (tx, rx) = channel();
                (f)(($($p.0,)*), tx);
                Pipeline(rx)
            }
        }
    };
}

impl_merge! { PA pa, PB pb }
impl_merge! { PA pa, PB pb, PC pc }
impl_merge! { PA pa, PB pb, PC pc, PD pd }
impl_merge! { PA pa, PB pb, PC pc, PD pd, PE pe }
impl_merge! { PA pa, PB pb, PC pc, PD pd, PE pe, PF pf }

/// A trait to merge multiple pipelines into one pipeline using fallible
/// closures.
pub trait MergeTry {
    /// Types of element receivers.
    type ReceiverTuple;

    /// Merge multiple pipelines into one pipeline.
    fn merge_try<E, T>(
        p: Self,
        f: impl FnOnce(Self::ReceiverTuple, Sender<T>) -> Result<(), E>,
    ) -> Result<Pipeline<T>, E>;
}

/// Implements [`MergeTry`].
macro_rules! impl_merge_try {
    ($($t:ident $p:ident),*) => {
        impl <$($t),*> MergeTry for ($(Pipeline<$t>,)*) {
            type ReceiverTuple = ($(Receiver<$t>,)*);

            fn merge_try<E, T>(
                p: Self,
                f: impl FnOnce(Self::ReceiverTuple, Sender<T>) -> Result<(), E>,
            ) -> Result<Pipeline<T>, E> {
                let ($($p,)*) = p;
                let (tx, rx) = channel();
                (f)(($($p.0,)*), tx)?;
                Ok(Pipeline(rx))
            }
        }
    };
}

impl_merge_try! { PA pa, PB pb }
impl_merge_try! { PA pa, PB pb, PC pc }
impl_merge_try! { PA pa, PB pb, PC pc, PD pd }
impl_merge_try! { PA pa, PB pb, PC pc, PD pd, PE pe }
impl_merge_try! { PA pa, PB pb, PC pc, PD pd, PE pe, PF pf }

/// A trait to fork a pipeline into multiple pipelines.
pub trait Fork
where
    Self: Sized,
{
    /// Types of output pipelines.
    type PipelineTuple;

    /// Fork a pipeline into multiple pipelines.
    fn fork<T>(p: Pipeline<T>, f: impl FnOnce(Receiver<T>, Self)) -> Self::PipelineTuple;
}

/// Implements [`Fork`].
macro_rules! impl_fork {
    ($($t:ident $tx:ident $rx:ident),*) => {
        impl <$($t),*> Fork for ($(Sender<$t>,)*) {
            type PipelineTuple = ($(Pipeline<$t>,)*);

            fn fork<T>(
                p: Pipeline<T>,
                f: impl FnOnce(Receiver<T>, Self),
            ) -> Self::PipelineTuple {
                $(
                    let ($tx, $rx) = channel();
                )*
                (f)(p.0, ($($tx,)*));
                ($(Pipeline($rx),)*)
            }
        }
    };
}

impl_fork! { PA ta ra, PB tb rb }
impl_fork! { PA ta ra, PB tb rb, PC tc rc }
impl_fork! { PA ta ra, PB tb rb, PC tc rc, PD td rd }
impl_fork! { PA ta ra, PB tb rb, PC tc rc, PD td rd, PE te re }
impl_fork! { PA ta ra, PB tb rb, PC tc rc, PD td rd, PE te re, PF tf rf }

/// A trait to fork a pipeline into multiple pipelines using fallible closures.
pub trait ForkTry
where
    Self: Sized,
{
    /// Types of output pipelines.
    type PipelineTuple;

    /// Fork a pipeline into multiple pipelines.
    fn fork_try<E, T>(
        p: Pipeline<T>,
        f: impl FnOnce(Receiver<T>, Self) -> Result<(), E>,
    ) -> Result<Self::PipelineTuple, E>;
}

/// Implements [`ForkTry`].
macro_rules! impl_fork_try {
    ($($t:ident $tx:ident $rx:ident),*) => {
        impl <$($t),*> ForkTry for ($(Sender<$t>,)*) {
            type PipelineTuple = ($(Pipeline<$t>,)*);

            fn fork_try<E, T>(
                p: Pipeline<T>,
                f: impl FnOnce(Receiver<T>, Self) -> Result<(), E>,
            ) -> Result<Self::PipelineTuple, E> {
                $(
                    let ($tx, $rx) = channel();
                )*
                (f)(p.0, ($($tx,)*))?;
                Ok(($(Pipeline($rx),)*))
            }
        }
    };
}

impl_fork_try! { PA ta ra, PB tb rb }
impl_fork_try! { PA ta ra, PB tb rb, PC tc rc }
impl_fork_try! { PA ta ra, PB tb rb, PC tc rc, PD td rd }
impl_fork_try! { PA ta ra, PB tb rb, PC tc rc, PD td rd, PE te re }
impl_fork_try! { PA ta ra, PB tb rb, PC tc rc, PD td rd, PE te re, PF tf rf }
