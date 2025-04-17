use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::Stream;

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
