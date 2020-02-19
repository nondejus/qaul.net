use crate::Identity;
use futures::{
    channel::mpsc::{channel, Sender, Receiver},
    task::{Context, Poll},
    stream::Stream,
};
use std::pin::Pin;

/// A unique, randomly generated subscriber ID
pub type SubId = Identity;

/// A generic subscription which can stream data from libqaul
///
/// Each subscription has a unique ID that can later on be used to
/// cancel the stream.  This type also allows for stream manipulation,
/// for example throttling throughput, or only taking a subset.
pub struct Subscription<T> {
    /// The subscription ID
    pub id: SubId,
    rx: Receiver<T>,
}

impl<T> Subscription<T> {
    /// Create a new subscription stream
    pub(crate) fn new() -> (Self, Sender<T>) {
        let (tx, rx) = channel(1);
        let id = SubId::random();
        (Self { id, rx }, tx)
    }
}

impl<T> Stream for Subscription<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut Pin::into_inner(self).rx).poll_next(ctx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rx.size_hint()
    }
}
