use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};

use crate::bridge::{Bridge, UnseenByClone};

/// A stream that implements `Clone` and returns cloned items from a base
/// stream.
pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    bridge: Arc<RwLock<Bridge<BaseStream>>>,
    pub id: usize,
}

impl<BaseStream> CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn active(&self) -> bool {
        let bridge = self.bridge.read().unwrap();
        bridge.clones.get(&self.id).unwrap().suspended_task.active()
    }

    #[must_use]
    pub fn queued_items(&self) -> usize {
        let bridge = self.bridge.read().unwrap();
        bridge.clones.get(&self.id).unwrap().unseen_items.len()
    }
}

impl<BaseStream> From<Bridge<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut bridge: Bridge<BaseStream>) -> Self {
        bridge.clones.insert(0, UnseenByClone::default());

        Self {
            id: 0,
            bridge: Arc::new(RwLock::new(bridge)),
        }
    }
}

impl<BaseStream> Clone for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        let mut bridge = self.bridge.write().unwrap();
        let min_available = (0..)
            .filter(|n| !bridge.clones.contains_key(n))
            .nth(0)
            .unwrap();
        bridge
            .clones
            .insert(min_available, UnseenByClone::default());
        drop(bridge);

        Self {
            bridge: self.bridge.clone(),
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
        let mut bridge = self.bridge.write().unwrap();
        bridge.poll(self.id, waker)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let bridge = self.bridge.read().unwrap();
        let (lower, upper) = bridge.base_stream.size_hint();
        let n_cached = bridge.clones.get(&self.id).unwrap().unseen_items.len();
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn is_terminated(&self) -> bool {
        let bridge = self.bridge.read().unwrap();
        bridge.base_stream.is_terminated()
            && bridge.clones.get(&self.id).unwrap().unseen_items.is_empty()
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut bridge = self.bridge.write().unwrap();
        bridge.clones.remove(&self.id);
    }
}
