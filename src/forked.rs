use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::Stream;
use log::trace;

use crate::{
    bridge::{ForkBridge, OutputStreamId},
    fork_stage::ForkStage,
};

/// A wrapper around a stream that implements `Clone`.
pub struct ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub output_index: OutputStreamId,
    clone_bridge: Arc<Mutex<ForkBridge<BaseStream>>>,
}

impl<BaseStream> From<ForkBridge<BaseStream>> for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(mut bridge: ForkBridge<BaseStream>) -> Self {
        let free_output_index = bridge.free_output_index();
        bridge
            .outputs
            .insert(free_output_index, ForkStage::default());
        Self {
            output_index: free_output_index,
            clone_bridge: Arc::new(Mutex::new(bridge)),
        }
    }
}

impl<BaseStream> Clone for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        let mut bridge = self.clone_bridge.lock().unwrap();
        let free_index = bridge.free_output_index();
        bridge.outputs.insert(free_index, ForkStage::default());
        Self {
            output_index: free_index,
            clone_bridge: self.clone_bridge.clone(),
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
        let mut bridge = self.clone_bridge.lock().unwrap();

        let mut status = bridge.outputs.get_mut(&self.output_index).unwrap();

        match &mut status {
            ForkStage::WakingUp(waking) => {
                let item = waking.processed.clone();
                waking
                    .remaining
                    .retain(|already_waiting| !already_waiting.will_wake(new_context.waker()));

                if waking.remaining.is_empty() {
                    bridge.outputs.remove(&self.output_index);
                }

                Poll::Ready(item)
            }
            ForkStage::Waiting(_) => {
                trace!(
                    "The output stream {} is waiting for the input stream to resolve.",
                    self.output_index
                );
                bridge.wake_others_or_queue(self.output_index, new_context.waker())
            }
        }
    }
}
