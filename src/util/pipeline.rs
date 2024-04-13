//! Execute pipelines.

use std::sync::mpsc::channel;
pub use std::sync::mpsc::{Receiver, SendError, Sender};

/// A pipeline.
#[derive(Debug)]
pub struct Pipeline<T>(Receiver<T>);

impl<T> Pipeline<T> {
    /// Create a pipeline from a task that takes one sender.
    pub fn new<E, F>(f: F) -> Result<Self, E>
    where
        F: Task<(), (T,), E>,
    {
        let (tx, rx) = channel();
        f.process((), (tx,))?;
        Ok(Self(rx))
    }

    /// Call a task that takes one receiver and one sender.
    pub fn pipe<U, E, F>(self, f: F) -> Result<Pipeline<U>, E>
    where
        F: Task<(T,), (U,), E>,
    {
        let (tx, rx) = channel();
        f.process((self.0,), (tx,))?;
        Ok(Pipeline(rx))
    }

    /// Call a task that takes one receiver and multiple senders.
    pub fn fork<S, E, F>(self, f: F) -> Result<<Self as Fork<T, S>>::Output, E>
    where
        Self: Fork<T, S>,
        F: Task<(T,), S, E>,
        S: Tuple,
    {
        Fork::fork(self, f)
    }

    /// Call a task that takes multiple receivers and one sender.
    pub fn merge<R, E, P, F>(p: P, f: F) -> Result<Self, E>
    where
        P: Merge<R, T>,
        F: Task<R, (T,), E>,
        R: Tuple,
    {
        P::merge(p, f)
    }

    /// Call a closure on each element of the pipeline.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Pipeline<U> {
        self.pipe(
            |rx: Receiver<T>, tx: Sender<U>| -> Result<(), SendError<U>> {
                for t in rx {
                    tx.send((f)(t))?;
                }
                Ok(())
            },
        )
        .unwrap()
    }

    /// Call a closure to determine if an element should be sent to the next
    /// stage.
    pub fn filter(self, mut f: impl FnMut(&T) -> bool) -> Self {
        self.pipe(
            |rx: Receiver<T>, tx: Sender<T>| -> Result<(), SendError<T>> {
                for t in rx {
                    if (f)(&t) {
                        tx.send(t)?;
                    }
                }
                Ok(())
            },
        )
        .unwrap()
    }

    /// Call a closure to both filter and map elements.
    pub fn filter_map<U>(self, mut f: impl FnMut(T) -> Option<U>) -> Pipeline<U> {
        self.pipe(
            |rx: Receiver<T>, tx: Sender<U>| -> Result<(), SendError<U>> {
                for t in rx {
                    if let Some(u) = (f)(t) {
                        tx.send(u)?;
                    }
                }
                Ok(())
            },
        )
        .unwrap()
    }
}

impl<T> FromIterator<T> for Pipeline<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(|tx: Sender<T>| -> Result<(), SendError<T>> {
            for t in iter {
                tx.send(t)?;
            }
            Ok(())
        })
        .unwrap()
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

/// A trait to fork one pipeline into multiple pipelines.
pub trait Fork<T, S>
where
    S: Tuple,
{
    /// Type of the [`Fork::fork`] method.
    type Output;

    /// Fork one pipeline into multiple pipelines.
    fn fork<F, E>(self, f: F) -> Result<Self::Output, E>
    where
        F: Task<(T,), S, E>;
}

/// Implements [`Fork`] for closures.
macro_rules! impl_fork {
    ($($t:ident $r:ident $s:ident),*) => {
        impl <T, $($t,)*> Fork<T, ($($t,)*)> for Pipeline<T> {
            type Output = ($(Pipeline<$t>,)*);

            fn fork<F, E>(self, f: F) -> Result<Self::Output, E>
            where
                F: Task<(T,), ($($t,)*), E>,
            {
                $(
                    let ($s, $r) = channel();
                )*
                f.process((self.0,), ($($s,)*))?;
                Ok(($(Pipeline($r),)*))
            }
        }
    };
}

impl_fork! { SA ra sa, SB rb sb }
impl_fork! { SA ra sa, SB rb sb, SC rc sc }
impl_fork! { SA ra sa, SB rb sb, SC rc sc, SD rd sd }
impl_fork! { SA ra sa, SB rb sb, SC rc sc, SD rd sd, SE re se }
impl_fork! { SA ra sa, SB rb sb, SC rc sc, SD rd sd, SE re se, SF rf sf }

/// A trait to merge multiple pipelines into one pipeline.
pub trait Merge<R, T>
where
    R: Tuple,
{
    /// Merge multiple pipelines into one pipeline.
    fn merge<F, E>(p: Self, f: F) -> Result<Pipeline<T>, E>
    where
        F: Task<R, (T,), E>;
}

/// Implements [`Merge`] for closures.
macro_rules! impl_merge {
    ($($t:ident $p:ident),*) => {
        impl <T, $($t,)*> Merge<($($t,)*), T> for ($(Pipeline<$t>,)*) {
            fn merge<F, E>(($($p,)*): Self, f: F) -> Result<Pipeline<T>, E>
            where
                F: Task<($($t,)*), (T,), E>,
            {
                let (tx, rx) = channel();
                f.process(($($p.0,)*), (tx,))?;
                Ok(Pipeline(rx))
            }
        }
    };
}

impl_merge! { RA pa, RB pb }
impl_merge! { RA pa, RB pb, RC pc }
impl_merge! { RA pa, RB pb, RC pc, RD pd }
impl_merge! { RA pa, RB pb, RC pc, RD pd, RE pe }
impl_merge! { RA pa, RB pb, RC pc, RD pd, RE pe, RF pf }

/// A pipeline task.
pub trait Task<R, S, E>
where
    R: Tuple,
    S: Tuple,
{
    /// Execute the task.
    fn process(self, rxs: R::Receivers, txs: S::Senders) -> Result<(), E>;
}

/// Implements [`Task`] for closures.
macro_rules! impl_task {
    (($($r:ident $ri:ident),*), ($($s:ident $si:ident),*)) => {
        impl <$($r,)* $($s,)* E, F> Task<($($r,)*), ($($s,)*), E> for F
        where
            F: FnOnce($(Receiver<$r>,)* $(Sender<$s>,)*) -> Result<(), E>,
        {
            fn process(self, ($($ri,)*): ($(Receiver<$r>,)*), ($($si,)*): ($(Sender<$s>,)*))
                -> Result<(), E>
            {
                (self)($($ri,)* $($si,)*)
            }
        }
    };
}

// Pipeline::new()
impl_task! { (), (SA sa) }
impl_task! { (), (SA sa, SB sb) }
impl_task! { (), (SA sa, SB sb, SC sc) }
impl_task! { (), (SA sa, SB sb, SC sc, SD sd) }
impl_task! { (), (SA sa, SB sb, SC sc, SD sd, SE se) }
impl_task! { (), (SA sa, SB sb, SC sc, SD sd, SE se, SF sf) }

// Pipeline::pipe()
impl_task! { (RA ra), (SA sa) }

// Pipeline::fork()
impl_task! { (RA ra), (SA sa, SB sb) }
impl_task! { (RA ra), (SA sa, SB sb, SC sc) }
impl_task! { (RA ra), (SA sa, SB sb, SC sc, SD sd) }
impl_task! { (RA ra), (SA sa, SB sb, SC sc, SD sd, SE se) }
impl_task! { (RA ra), (SA sa, SB sb, SC sc, SD sd, SE se, SF sf) }

// Pipeline::merge()
impl_task! { (RA ra, RB rb), (SA sa) }
impl_task! { (RA ra, RB rb, RC rc), (SA sa) }
impl_task! { (RA ra, RB rb, RC rc, RD rd), (SA sa) }
impl_task! { (RA ra, RB rb, RC rc, RD rd, RE re), (SA sa) }
impl_task! { (RA ra, RB rb, RC rc, RD rd, RE re, RF rf), (SA sa) }

/// A trait for representing tuples and deriving types.
pub trait Tuple {
    /// The tuple of [`Receiver`]s.
    type Receivers;

    /// The tuple of [`Sender`]s.
    type Senders;
}

/// Implements [`Receivers`] for arbitrary tuples.
macro_rules! impl_tuple {
    ($($t:ident),*) => {
        impl <$($t),*> Tuple for ($($t,)*) {
            type Receivers = ($(Receiver<$t>,)*);
            type Senders = ($(Sender<$t>,)*);
        }
    };
}

impl_tuple! {}
impl_tuple! { A }
impl_tuple! { A, B }
impl_tuple! { A, B, C }
impl_tuple! { A, B, C, D }
impl_tuple! { A, B, C, D, E }
impl_tuple! { A, B, C, D, E, F }
