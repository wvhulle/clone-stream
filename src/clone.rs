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
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fork: Arc<RwLock<Fork<BaseStream>>>,
    pub id: usize,
}

impl<BaseStream> From<Fork<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut fork: Fork<BaseStream>) -> Self {
        let id = fork.register();

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
    fn clone(&self) -> Self {
        let mut fork = self.fork.write().unwrap();
        let min_available = fork.register();
        drop(fork);

        Self {
            fork: self.fork.clone(),
            id: min_available,
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
        let mut fork = self.fork.write().unwrap();
        fork.poll_clone(self.id, waker)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let fork = self.fork.read().unwrap();
        let (lower, upper) = fork.size_hint();
        let n_cached = fork.remaining_queued_items(self.id);
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: FusedStream<Item: Clone>,
{
    fn is_terminated(&self) -> bool {
        let fork = self.fork.read().unwrap();

        fork.is_terminated() && fork.remaining_queued_items(self.id) == 0
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut fork = self.fork.write().unwrap();
        fork.unregister(self.id);
    }
}

impl<BaseStream> CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn n_queued_items(&self) -> usize {
        trace!("Getting the number of queued items for clone {}.", self.id);
        self.fork.read().unwrap().remaining_queued_items(self.id)
    }
}
