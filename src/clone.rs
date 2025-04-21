use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};

use crate::fork::{CloneTaskState, Fork};

/// A stream that implements `Clone` and returns cloned items from a base
/// stream.
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    split: Arc<RwLock<Fork<BaseStream>>>,
    pub id: usize,
}

impl<BaseStream> From<Fork<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut split: Fork<BaseStream>) -> Self {
        split.clones.insert(0, CloneTaskState::default());

        Self {
            id: 0,
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
        let min_available = (0..)
            .filter(|n| !split.clones.contains_key(n))
            .nth(0)
            .unwrap();
        split
            .clones
            .insert(min_available, CloneTaskState::default());
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
        split.update(self.id, waker)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let split = self.split.read().unwrap();
        let (lower, upper) = split.base_stream.size_hint();
        let n_cached = split.n_queued_items(self.id);
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn is_terminated(&self) -> bool {
        let split = self.split.read().unwrap();

        split.base_stream.is_terminated() && split.n_queued_items(self.id) == 0
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut split = self.split.write().unwrap();
        split.clones.remove(&self.id);
    }
}

impl<BaseStream> CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn polled_once(&self) -> bool {
        self.split
            .read()
            .unwrap()
            .clones
            .get(&self.id)
            .unwrap()
            .polled_once()
    }

    #[must_use]
    pub fn n_queued_items(&self) -> usize {
        self.split.read().unwrap().n_queued_items(self.id)
    }
}
