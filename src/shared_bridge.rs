use std::sync::{Arc, Mutex};

use futures::Stream;

use crate::bridge::ForkBridge;

impl<BaseStream> From<ForkBridge<BaseStream>> for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        SharedBridge(Arc::new(Mutex::new(bridge)))
    }
}

pub struct SharedBridge<BaseStream>(pub Arc<Mutex<ForkBridge<BaseStream>>>)
where
    BaseStream: Stream<Item: Clone>;

impl<BaseStream> Clone for SharedBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        SharedBridge(self.0.clone())
    }
}
