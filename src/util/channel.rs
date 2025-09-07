//! Channel utilities.

use async_channel::Receiver;

/// Extend [`async_channel::Receiver`].
pub trait ReceiverExt<T> {
    fn into_iter(self) -> impl Iterator<Item = T>;
}

impl<T> ReceiverExt<T> for Receiver<T> {
    fn into_iter(self) -> impl Iterator<Item = T> {
        struct Iter<T>(Receiver<T>);

        impl<T> Iterator for Iter<T> {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.recv_blocking().ok()
            }
        }

        Iter(self)
    }
}
