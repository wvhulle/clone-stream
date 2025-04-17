use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};

use crate::split::{CloneTaskState, Split};

/// A stream that implements `Clone` and returns cloned items from a base
/// stream.
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    split: Arc<RwLock<Split<BaseStream>>>,
    pub id: usize,
}

impl<BaseStream> From<Split<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut split: Split<BaseStream>) -> Self {
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
        let n_cached = split.clones.get(&self.id).unwrap().max_size();
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn is_terminated(&self) -> bool {
        let split = self.split.read().unwrap();

        split.base_stream.is_terminated() && split.clones.get(&self.id).unwrap().max_size() == 0
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
    pub fn active(&self) -> bool {
        self.split
            .read()
            .unwrap()
            .clones
            .values()
            .any(super::split::CloneTaskState::active)
    }

    #[must_use]
    pub fn queued_items(&self) -> usize {
        self.split
            .read()
            .unwrap()
            .clones
            .get(&self.id)
            .unwrap()
            .max_size()
    }
}
