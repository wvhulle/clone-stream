use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};
use log::trace;

use crate::fork::Fork;

/// A stream that implements `Clone` and returns cloned items from a base
/// stream.
///
/// This is the main type provided by this crate. It wraps any [`Stream`] whose
/// items implement [`Clone`], allowing the stream itself to be cloned. Each
/// clone operates independently but shares the same underlying stream data.
///
/// # Examples
///
/// ```rust
/// use clone_stream::ForkStream;
/// use futures::{StreamExt, stream};
///
/// # #[tokio::main]
/// # async fn main() {
/// let stream = stream::iter(vec![1, 2, 3]);
/// let clone_stream = stream.fork();
///
/// // Create multiple clones that can be used independently
/// let mut clone1 = clone_stream.clone();
/// let mut clone2 = clone_stream.clone();
///
/// // Each clone can be polled independently
/// let item1 = clone1.next().await;
/// let item2 = clone2.next().await;
///
/// println!("Clone1 got: {:?}, Clone2 got: {:?}", item1, item2);
/// # }
/// ```
///
/// # Performance
///
/// Items are cached internally until all clones have consumed them. The memory
/// usage grows with the number of items that haven't been consumed by all
/// clones yet.
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fork: Arc<RwLock<Fork<BaseStream>>>,
    /// Unique identifier for this clone within the fork
    pub id: usize,
}

impl<BaseStream> From<Fork<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut fork: Fork<BaseStream>) -> Self {
        let id = fork.register().expect("Failed to register initial clone");

        Self {
            id,
            fork: Arc::new(RwLock::new(fork)),
        }
    }
}

impl<BaseStream> Clone for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    /// Creates a new clone of this stream.
    ///
    /// # Panics
    ///
    /// Panics if the maximum number of clones has been exceeded for this
    /// stream. The limit is set when creating the stream with
    /// [`ForkStream::fork_with_limits`].
    ///
    /// [`ForkStream::fork_with_limits`]: crate::ForkStream::fork_with_limits
    fn clone(&self) -> Self {
        let mut fork = self.fork.write().expect("Fork lock poisoned during clone");
        let clone_id = fork
            .register()
            .expect("Failed to register clone - clone limit exceeded");
        drop(fork);

        Self {
            fork: self.fork.clone(),
            id: clone_id,
        }
    }
}

impl<BaseStream> Stream for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Item = BaseStream::Item;

    fn poll_next(self: Pin<&mut Self>, current_task: &mut Context) -> Poll<Option<Self::Item>> {
        let waker = current_task.waker();
        let mut fork = self
            .fork
            .write()
            .expect("Fork lock poisoned during poll_next");
        fork.poll_clone(self.id, waker)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let fork = self
            .fork
            .read()
            .expect("Fork lock poisoned during size_hint");
        let (lower, upper) = fork.size_hint();
        let n_cached = fork.remaining_queued_items(self.id);
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: FusedStream<Item: Clone>,
{
    /// Returns `true` if the stream is terminated.
    ///
    /// A clone stream is considered terminated when both:
    /// 1. The underlying base stream is terminated
    /// 2. This clone has no remaining queued items to consume
    fn is_terminated(&self) -> bool {
        let fork = self
            .fork
            .read()
            .expect("Fork lock poisoned during is_terminated");
        fork.is_terminated() && fork.remaining_queued_items(self.id) == 0
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        if let Ok(mut fork) = self.fork.try_write() {
            fork.unregister(self.id);
        } else {
            log::warn!(
                "Failed to acquire lock during clone drop for clone {}",
                self.id
            );
        }
    }
}

impl<BaseStream> CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    /// Returns the number of items currently queued for this clone.
    ///
    /// This represents items that have been produced by the base stream but not
    /// yet consumed by this particular clone. Other clones may have
    /// different queue lengths depending on their consumption patterns.
    ///
    /// # Panics
    ///
    /// Panics if the internal fork lock is poisoned. This should not happen
    /// under normal circumstances.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use clone_stream::ForkStream;
    /// use futures::stream;
    ///
    /// let stream = stream::iter(vec![1, 2, 3]);
    /// let clone_stream = stream.fork();
    /// assert_eq!(clone_stream.n_queued_items(), 0);
    /// ```
    #[must_use]
    pub fn n_queued_items(&self) -> usize {
        trace!("Getting the number of queued items for clone {}.", self.id);
        self.fork
            .read()
            .expect("Fork lock poisoned during n_queued_items")
            .remaining_queued_items(self.id)
    }
}
