use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::Waker,
};

use futures::Stream;
use log::trace;

pub type OutputStreamId = usize;

#[derive(Clone)]
pub enum WakerStatus<Item> {
    WaitingForItemToArrive(VecDeque<Waker>),
    WaitingUntilWakersWoken {
        finally_arrived_item: Option<Item>,
        remaining_tasks_to_wake: VecDeque<Waker>,
    },
    NotWaiting,
}

pub(crate) struct ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(super) stream: Pin<Box<BaseStream>>,
    pub(super) outputs: BTreeMap<OutputStreamId, WakerStatus<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn free_output_index(&self) -> OutputStreamId {
        if self.outputs.is_empty() {
            0
        } else {
            self.outputs.keys().max().unwrap() + 1
        }
    }

    pub fn notify_waiters(&mut self, id: OutputStreamId, maybe_item: Option<BaseStream::Item>) {
        trace!(
            "A poll on the output stream {} triggered a result from the input stream.",
            id
        );
        if self.outputs.is_empty() {
            trace!("No output streams are waiting for the input stream to resolve an item.");
        } else {
            trace!("Updating all output streams.");
        }

        self.outputs
            .iter_mut()
            .for_each(|(id, mut status)| match &mut status {
                WakerStatus::NotWaiting => {
                    trace!("The output stream {} does not have any tasks waiting for its output.", id);
                },
                WakerStatus::WaitingForItemToArrive(waker) => {
                    trace!("Waking up a waker stored for output stream {}.", id);
                    let waiting_wakers = waker.clone();
                    *status = WakerStatus::WaitingUntilWakersWoken {
                        finally_arrived_item: maybe_item.clone(),
                        remaining_tasks_to_wake: waker.clone(),
                    };
                    waiting_wakers.into_iter().for_each(Waker::wake);
                }
                WakerStatus::WaitingUntilWakersWoken {
                    finally_arrived_item,
                    remaining_tasks_to_wake,
                } => {
                    if remaining_tasks_to_wake.is_empty() {
                        trace!("All tasks have been woken up for output stream {}, going back to not waiting.", id);
                        *status = WakerStatus::NotWaiting;
                    } else {
                        trace!("There are still tasks to wake up for output stream {}." , id);
                        *finally_arrived_item = maybe_item.clone();
                        remaining_tasks_to_wake.iter().for_each(Waker::wake_by_ref);
                        remaining_tasks_to_wake.clear();
                    }
                }
            });
    }
}

impl<BaseStream> From<BaseStream> for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(stream: BaseStream) -> Self {
        Self {
            stream: Box::pin(stream),
            outputs: BTreeMap::new(),
        }
    }
}
