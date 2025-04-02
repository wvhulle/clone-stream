use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};
use log::trace;

use crate::bridge::{ForkBridge, OutputStreamId, WakerStatus};

/// A wrapper around a stream that implements `Clone`.
pub struct ForkedStream<InputStream>
where
    InputStream: Stream<Item: Clone>,
{
    pub output_index: OutputStreamId,
    clone_bridge: Arc<Mutex<ForkBridge<InputStream>>>,
}

impl<InputStream> From<ForkBridge<InputStream>> for ForkedStream<InputStream>
where
    InputStream: Stream<Item: Clone>,
{
    fn from(mut bridge: ForkBridge<InputStream>) -> Self {
        let free_output_index = bridge.free_output_index();
        bridge
            .outputs
            .insert(free_output_index, WakerStatus::NotWaiting);
        Self {
            output_index: free_output_index,
            clone_bridge: Arc::new(Mutex::new(bridge)),
        }
    }
}

impl<S> Clone for ForkedStream<S>
where
    S: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        let mut bridge = self.clone_bridge.lock().unwrap();
        let free_index = bridge.free_output_index();
        bridge.outputs.insert(free_index, WakerStatus::NotWaiting);

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
        let mut bridge = self.clone_bridge.lock().unwrap();
        match bridge.outputs.remove(&self.output_index) {
            Some(status) => match status {
                WakerStatus::NotWaiting => {
                    trace!(
                        "No waiting tasks registered for output stream {}.",
                        self.output_index
                    );

                    match bridge.stream.poll_next_unpin(new_context) {
                        Poll::Pending => {
                            trace!(
                                "The input stream is not ready yet. Marking the output stream with id {} as waiting.",
                                self.output_index
                            );
                            bridge.outputs.insert(
                                self.output_index,
                                WakerStatus::WaitingForItemToArrive(VecDeque::from([new_context
                                    .waker()
                                    .clone()])),
                            );
                            Poll::Pending
                        }
                        Poll::Ready(maybe_item) => {
                            bridge.outputs.insert(self.output_index, status);

                            bridge.notify_waiters(self.output_index, maybe_item.clone());

                            Poll::Ready(maybe_item)
                        }
                    }
                }
                WakerStatus::WaitingUntilWakersWoken {
                    finally_arrived_item,
                    mut remaining_tasks_to_wake,
                } => {
                    remaining_tasks_to_wake
                        .retain(|old_waker| !old_waker.will_wake(new_context.waker()));

                    if remaining_tasks_to_wake.is_empty() {
                        trace!(
                            "No tasks remain to wake, setting status to not waiting for output stream {}.",
                            self.output_index
                        );
                        bridge
                            .outputs
                            .insert(self.output_index, WakerStatus::NotWaiting);
                    } else {
                        bridge.outputs.insert(
                            self.output_index,
                            WakerStatus::WaitingUntilWakersWoken {
                                finally_arrived_item: finally_arrived_item.clone(),
                                remaining_tasks_to_wake,
                            },
                        );
                    }

                    Poll::Ready(finally_arrived_item.clone())
                }
                WakerStatus::WaitingForItemToArrive(mut already_waiting_tasks) => {
                    let new_waker = new_context.waker().clone();

                    trace!(
                        "This is a new task that is waiting for output stream {}.",
                        self.output_index
                    );
                    already_waiting_tasks.push_back(new_waker);

                    bridge.outputs.insert(
                        self.output_index,
                        WakerStatus::WaitingForItemToArrive(already_waiting_tasks),
                    );

                    Poll::Pending
                }
            },
            None => match bridge.stream.poll_next_unpin(new_context) {
                Poll::Ready(maybe_item) => {
                    trace!(
                        "The input stream has resolved an item. Emitting it to all output streams."
                    );

                    bridge
                        .outputs
                        .insert(self.output_index, WakerStatus::NotWaiting);

                    bridge.notify_waiters(self.output_index, maybe_item.clone());

                    Poll::Ready(maybe_item)
                }
                Poll::Pending => {
                    trace!(
                        "The input stream is not ready yet. Marking the output stream with id {} as waiting.",
                        self.output_index
                    );
                    bridge.outputs.insert(
                        self.output_index,
                        WakerStatus::WaitingForItemToArrive(VecDeque::from([new_context
                            .waker()
                            .clone()])),
                    );
                    Poll::Pending
                }
            },
        }
    }
}
