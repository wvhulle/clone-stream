// A stream that implements `Clone` and takes input from the `BaseStream`i
use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};

use crate::{
    Fork,
    bridge::{Bridge, ForkRef},
};

pub struct CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    bridge: Fork<BaseStream>,
    pub id: usize,
}

impl<BaseStream> From<Fork<BaseStream>> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(fork: Fork<BaseStream>) -> Self {
        let mut bridge = fork.write().unwrap();
        bridge.forks.insert(0, ForkRef::default());
        drop(bridge);
        Self {
            id: 0,
            bridge: fork,
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
            .filter(|n| !bridge.forks.contains_key(n))
            .nth(0)
            .unwrap();
        bridge.forks.insert(min_available, ForkRef::default());
        drop(bridge);

        Self {
            bridge: self.bridge.clone(),
            id: min_available,
        }
    }
}

impl<BaseStream> Deref for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Arc<RwLock<Bridge<BaseStream>>>;

    fn deref(&self) -> &Self::Target {
        &self.bridge
    }
}

impl<BaseStream> DerefMut for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bridge
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
        let n_cached = bridge.forks.get(&self.id).unwrap().items.len();
        (lower + n_cached, upper.map(|u| u + n_cached))
    }
}

impl<BaseStream> FusedStream for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone> + FusedStream,
{
    fn is_terminated(&self) -> bool {
        let bridge = self.bridge.read().unwrap();

        bridge.is_terminated() && bridge.forks.get(&self.id).unwrap().items.is_empty()
    }
}

impl<BaseStream> Drop for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut bridge = self.bridge.write().unwrap();
        bridge.forks.remove(&self.id);
    }
}
