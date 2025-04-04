use std::sync::{Arc, RwLock};

use futures::Stream;
use log::warn;

use crate::bridge::ForkBridge;

impl<BaseStream> From<ForkBridge<BaseStream>> for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        SharedBridge(Arc::new(RwLock::new(bridge)))
    }
}

pub struct SharedBridge<BaseStream>(pub Arc<RwLock<ForkBridge<BaseStream>>>)
where
    BaseStream: Stream<Item: Clone>;

impl<BaseStream> SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn modify<R>(&self, modify: impl FnOnce(&mut ForkBridge<BaseStream>) -> R) -> R {
        match self.0.write() {
            Ok(mut bridge) => modify(&mut bridge),
            Err(mut e) => {
                warn!("The previous task who locked the bridge to modify it panicked");
                modify(e.get_mut())
            }
        }
    }

    pub fn get<R>(&self, get: impl FnOnce(&ForkBridge<BaseStream>) -> R) -> R {
        match self.0.read() {
            Ok(bridge) => get(&bridge),
            Err(e) => {
                warn!("The previous task who locked the bridge to read it panicked");
                get(e.get_ref())
            }
        }
    }
}

impl<BaseStream> SharedBridge<BaseStream> where BaseStream: Stream<Item: Clone> {}

impl<BaseStream> Clone for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        SharedBridge(self.0.clone())
    }
}
