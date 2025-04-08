// A stream that implements `Clone` and takes input from the `BaseStream`i
use std::{
    collections::VecDeque,
    ops::Deref,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};
use log::warn;

use crate::bridge::{ForkBridge, ForkRef};

pub struct ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    bridge: Arc<RwLock<ForkBridge<BaseStream>>>,
    id: usize,
}
impl<BaseStream> ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn new(mut bridge: ForkBridge<BaseStream>) -> Self {
        bridge.forks.clear();
        bridge.forks.insert(0, ForkRef::default());
        Self {
            id: 0,
            bridge: Arc::new(RwLock::new(bridge)),
        }
    }

    pub fn modify<R>(&self, modify: impl FnOnce(&mut ForkBridge<BaseStream>) -> R) -> R {
        match self.write() {
            Ok(mut bridge) => modify(&mut bridge),
            Err(mut poisened) => {
                warn!("The previous task who locked the bridge to modify it panicked");
                let corrupted_bridge = poisened.get_mut();
                corrupted_bridge.clear();
                modify(corrupted_bridge)
            }
        }
    }

    pub fn get<R>(&self, get: impl FnOnce(&ForkBridge<BaseStream>) -> R) -> R {
        match self.read() {
            Ok(bridge) => get(&bridge),
            Err(e) => {
                warn!("The previous task who locked the bridge to read it panicked");

                get(e.get_ref())
            }
        }
    }
}

impl<BaseStream> From<ForkBridge<BaseStream>> for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        Self::new(bridge)
    }
}

impl<BaseStream> Clone for ForkedStream<BaseStream>
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

impl<BaseStream> Deref for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Arc<RwLock<ForkBridge<BaseStream>>>;

    fn deref(&self) -> &Self::Target {
        &self.bridge
    }
}

impl<BaseStream> Stream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Item = BaseStream::Item;

    fn poll_next(self: Pin<&mut Self>, current_task: &mut Context) -> Poll<Option<Self::Item>> {
        let waker = current_task.waker();
        self.modify(|bridge| bridge.poll(self.id, waker))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.get(|bridge| {
            let (lower, upper) = bridge.base_stream.size_hint();
            let n_cached = bridge.forks.get(&self.id).unwrap().items.len();
            (lower + n_cached, upper.map(|u| u + n_cached))
        })
    }
}

impl<BaseStream> FusedStream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone> + FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.get(|base_stream| {
            base_stream.is_terminated() && base_stream.forks.get(&self.id).unwrap().items.is_empty()
        })
    }
}

impl<BaseStream> Drop for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        self.modify(|bridge| bridge.forks.remove(&self.id));
    }
}
