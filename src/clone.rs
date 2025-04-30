use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};
use log::trace;

use crate::{fork::Fork, next::Next};

/// A stream that implements `Clone` and returns cloned items from a base
/// stream.
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) split: Arc<RwLock<Fork<BaseStream>>>,
    pub id: usize,
}

impl<BaseStream> From<Fork<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut split: Fork<BaseStream>) -> Self {
        let id = split.register();

        Self {
            id,
            split: Arc::new(RwLock::new(split)),
        }
    }
}

impl<BaseStream> Clone for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        let mut split = self.split.write().unwrap();
        let min_available = split.register();
        drop(split);

        Self {
            split: self.split.clone(),
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
        let mut split = self.split.write().unwrap();
        split.poll_clone(self.id, waker)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let split = self.split.read().unwrap();
        let (lower, upper) = split.size_hint();
        let n_cached = split.remaining_queued_items(self.id);
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: FusedStream<Item: Clone>,
{
    fn is_terminated(&self) -> bool {
        let split = self.split.read().unwrap();

        split.is_terminated() && split.remaining_queued_items(self.id) == 0
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut split = self.split.write().unwrap();
        split.unregister(self.id);
    }
}

impl<BaseStream> CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn n_queued_items(&self) -> usize {
        trace!("Getting the number of queued items for clone {}.", self.id);
        self.split.read().unwrap().remaining_queued_items(self.id)
    }

    pub fn next(&mut self) -> Next<BaseStream> {
        Next::new(self)
    }
}
