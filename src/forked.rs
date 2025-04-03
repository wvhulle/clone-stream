use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::Stream;
use log::trace;

use crate::{
    bridge::{ForkBridge, ForkId},
    fork_stage::ForkStage,
};

/// A wrapper around a stream that implements `Clone`.
pub struct ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub output_index: ForkId,
    fork_bridge: Arc<ForkBridge<BaseStream>>,
}

impl<BaseStream> From<ForkBridge<BaseStream>> for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: ForkBridge<BaseStream>) -> Self {
        let free_output_index = bridge.new_fork();
        Self {
            output_index: free_output_index,
            fork_bridge: Arc::new(bridge),
        }
    }
}

impl<BaseStream> Clone for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        let free_index = self.fork_bridge.as_ref().new_fork();
        Self {
            output_index: free_index,
            fork_bridge: self.fork_bridge.clone(),
        }
    }
}

impl<BaseStream> Stream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Item = BaseStream::Item;

    fn poll_next(self: Pin<&mut Self>, new_context: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("Forked stream {} is being polled.", self.output_index);

        self.fork_bridge
            .handle_fork(self.output_index, new_context.waker())
    }
}
