use std::sync::{Arc, Mutex};

use futures::Stream;

use crate::{ForkedStream, bridge::ForkBridge};

impl<BaseStream> From<ForkBridge<BaseStream>> for CloneableForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        CloneableForkBridge(Arc::new(Mutex::new(bridge)))
    }
}

pub struct CloneableForkBridge<BaseStream>(pub Arc<Mutex<ForkBridge<BaseStream>>>)
where
    BaseStream: Stream<Item: Clone>;

impl<BaseStream> Clone for CloneableForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        CloneableForkBridge(self.0.clone())
    }
}

impl<BaseStream> CloneableForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(stream: BaseStream) -> Self {
        Self(Arc::new(Mutex::new(ForkBridge::from(stream))))
    }

    pub fn new_fork(&self) -> ForkedStream<BaseStream> {
        ForkedStream(CloneableForkBridge(self.0.clone()))
    }
}
