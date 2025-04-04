use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use futures::Stream;
use log::warn;

use crate::fork_bridge::ForkBridge;

pub struct SharedBridge<BaseStream>(Arc<RwLock<ForkBridge<BaseStream>>>)
where
    BaseStream: Stream<Item: Clone>;

impl<BaseStream> SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
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

impl<BaseStream> From<ForkBridge<BaseStream>> for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        SharedBridge(Arc::new(RwLock::new(bridge)))
    }
}

impl<BaseStream> Clone for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        SharedBridge(self.deref().clone())
    }
}

impl<BaseStream> Deref for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Arc<RwLock<ForkBridge<BaseStream>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
