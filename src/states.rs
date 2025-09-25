use std::{
    fmt::Debug,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::{debug, trace};

use crate::Fork;

/// Represents the state of a clone in the stream cloning state machine.
///
/// Each clone maintains its own state to track its position relative to the
/// base stream and the shared queue. The state determines how the clone should
/// behave when polled.
#[derive(Clone, Debug)]
pub(crate) enum CloneState {
    AwaitingFirstItem,
    BaseStreamReady,

    AwaitingBaseStream {
        waker: Waker,
    },

    AwaitingBaseStreamWithQueueHistory {
        waker: Waker,
        last_seen_index: usize,
    },

    BaseStreamReadyWithQueueHistory,
    ProcessingQueue {
        last_seen_queue_index: usize,
    },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::AwaitingFirstItem
    }
}

use CloneState::{
    AwaitingBaseStream, AwaitingBaseStreamWithQueueHistory, AwaitingFirstItem, BaseStreamReady,
    BaseStreamReadyWithQueueHistory, ProcessingQueue,
};

impl CloneState {
    pub(crate) fn should_still_see_base_item(&self) -> bool {
        trace!("Checking if clone in state {self:?} should still see base item");

        match self {
            AwaitingBaseStream { .. }
            | AwaitingBaseStreamWithQueueHistory { .. }
            | BaseStreamReady => true,
            AwaitingFirstItem | BaseStreamReadyWithQueueHistory | ProcessingQueue { .. } => false,
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            AwaitingBaseStream { waker } | AwaitingBaseStreamWithQueueHistory { waker, .. } => {
                Some(waker.clone())
            }
            AwaitingFirstItem
            | BaseStreamReady
            | BaseStreamReadyWithQueueHistory
            | ProcessingQueue { .. } => None,
        }
    }

    #[inline]
    fn transition_on_poll<Item>(
        &mut self,
        poll_result: Poll<Option<Item>>,
        ready_state: CloneState,
        pending_state: CloneState,
    ) -> Poll<Option<Item>> {
        match poll_result {
            Poll::Ready(item) => {
                *self = ready_state;
                Poll::Ready(item)
            }
            Poll::Pending => {
                *self = pending_state;
                Poll::Pending
            }
        }
    }
}

impl CloneState {
    #[inline]
    pub(crate) fn step<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match self {
            AwaitingFirstItem | BaseStreamReady => self.transition_on_poll(
                poll_base_with_queue_check(clone_id, waker, fork),
                BaseStreamReady,
                next_pending_state(waker, fork),
            ),
            AwaitingBaseStream { .. } => {
                if fork.item_buffer.is_empty() {
                    debug!("Clone {clone_id}: Queue still empty, polling base stream");
                    self.transition_on_poll(
                        poll_base_with_queue_check(clone_id, waker, fork),
                        BaseStreamReady,
                        AwaitingBaseStream {
                            waker: waker.clone(),
                        },
                    )
                } else {
                    debug!("Clone {clone_id}: Queue now has items, processing oldest");
                    let (oldest_queue_index, item) =
                        pop_or_clone_oldest_unseen_queue_item(fork, clone_id);
                    *self = ProcessingQueue {
                        last_seen_queue_index: oldest_queue_index,
                    };
                    Poll::Ready(item)
                }
            }
            AwaitingBaseStreamWithQueueHistory {
                last_seen_index, ..
            } => {
                let last_seen_index = *last_seen_index;
                if let Some((newer_index, item)) = process_newer_queue_item(fork, last_seen_index) {
                    *self = ProcessingQueue {
                        last_seen_queue_index: newer_index,
                    };
                    Poll::Ready(item)
                } else {
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        BaseStreamReadyWithQueueHistory,
                        AwaitingBaseStreamWithQueueHistory {
                            waker: waker.clone(),
                            last_seen_index,
                        },
                    )
                }
            }
            BaseStreamReadyWithQueueHistory => {
                let pending_state = if let Some(oldest_index) = fork.item_buffer.oldest_index() {
                    AwaitingBaseStreamWithQueueHistory {
                        waker: waker.clone(),
                        last_seen_index: oldest_index,
                    }
                } else {
                    AwaitingBaseStream {
                        waker: waker.clone(),
                    }
                };

                self.transition_on_poll(
                    poll_base_stream(clone_id, waker, fork),
                    BaseStreamReadyWithQueueHistory,
                    pending_state,
                )
            }
            ProcessingQueue {
                last_seen_queue_index,
            } => {
                let last_seen_queue_index = *last_seen_queue_index;
                trace!(
                    "Clone {clone_id}: previously a queue item was ready, checking if there is a newer one at {last_seen_queue_index}"
                );
                if let Some((newer_index, item)) =
                    process_newer_queue_item(fork, last_seen_queue_index)
                {
                    trace!("Clone {clone_id}: Found newer item at {newer_index}");
                    *self = ProcessingQueue {
                        last_seen_queue_index: newer_index,
                    };
                    Poll::Ready(item)
                } else {
                    trace!(
                        "Clone {clone_id}: No newer item, transitioning to BaseStreamReadyWithQueueHistory"
                    );
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        BaseStreamReadyWithQueueHistory,
                        AwaitingBaseStreamWithQueueHistory {
                            waker: waker.clone(),
                            last_seen_index: last_seen_queue_index,
                        },
                    )
                }
            }
        }
    }
}

#[inline]
pub(crate) fn poll_base_stream<BaseStream>(
    clone_id: usize,
    waker: &Waker,
    fork: &mut Fork<BaseStream>,
) -> Poll<Option<BaseStream::Item>>
where
    BaseStream: Stream<Item: Clone>,
{
    match fork
        .base_stream
        .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
    {
        Poll::Ready(item) => {
            trace!("Base stream ready with item");
            if fork.has_other_clones_waiting(clone_id) {
                trace!("Queuing item for other waiting clones");
                fork.item_buffer.push(item.clone());
            } else {
                trace!("No other clones waiting, not queuing item");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("Base stream pending");
            Poll::Pending
        }
    }
}

#[inline]
fn poll_base_with_queue_check<BaseStream>(
    clone_id: usize,
    waker: &Waker,
    fork: &mut Fork<BaseStream>,
) -> Poll<Option<BaseStream::Item>>
where
    BaseStream: Stream<Item: Clone>,
{
    match fork
        .base_stream
        .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
    {
        Poll::Ready(item) => {
            trace!("Base stream ready with item");

            if fork.has_other_clones_waiting(clone_id) {
                trace!("Queuing item for other interested clones");
                fork.item_buffer.push(item.clone());
            } else {
                trace!("No other clones need this item");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("Base stream pending");
            Poll::Pending
        }
    }
}

#[inline]
fn next_pending_state<BaseStream>(waker: &Waker, fork: &Fork<BaseStream>) -> CloneState
where
    BaseStream: Stream<Item: Clone>,
{
    use CloneState::{AwaitingBaseStream, AwaitingBaseStreamWithQueueHistory};
    if fork.item_buffer.is_empty() {
        AwaitingBaseStream {
            waker: waker.clone(),
        }
    } else if let Some(newest_index) = fork.item_buffer.newest {
        AwaitingBaseStreamWithQueueHistory {
            waker: waker.clone(),
            last_seen_index: newest_index,
        }
    } else {
        AwaitingBaseStream {
            waker: waker.clone(),
        }
    }
}

#[inline]
fn pop_or_clone_oldest_unseen_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    clone_id: usize,
) -> (usize, Option<BaseStream::Item>)
where
    BaseStream: Stream<Item: Clone>,
{
    let previous_occupied_oldest_queue_index = fork
        .item_buffer
        .oldest_index()
        .expect("Queue reported non-empty but has no oldest index - this is a bug in RingQueue");

    let other_clones_want_item =
        fork.clone_registry
            .iter_active_with_ids()
            .any(|(other_clone_id, _)| {
                other_clone_id != clone_id
                    && fork
                        .should_clone_see_item(other_clone_id, previous_occupied_oldest_queue_index)
            });

    let oldest_queue_item = if other_clones_want_item {
        fork.item_buffer
            .get(previous_occupied_oldest_queue_index)
            .unwrap()
            .clone()
    } else {
        fork.item_buffer.pop_oldest().unwrap()
    };

    (previous_occupied_oldest_queue_index, oldest_queue_item)
}

#[inline]
fn process_newer_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    last_seen_queue_index: usize,
) -> Option<(usize, Option<BaseStream::Item>)>
where
    BaseStream: Stream<Item: Clone>,
{
    let newer_index = fork
        .item_buffer
        .find_next_newer_index(last_seen_queue_index)?;

    let item = if fork.active_clone_count() <= 1 {
        fork.item_buffer.remove(newer_index).unwrap()
    } else {
        let clones_needing_item = fork
            .clone_registry
            .iter_active_with_ids()
            .filter(|(clone_id, _)| fork.should_clone_see_item(*clone_id, newer_index))
            .count();
        match clones_needing_item {
            0 | 1 => fork.item_buffer.remove(newer_index).unwrap(),
            _ => fork.item_buffer.get(newer_index).unwrap().clone(),
        }
    };

    Some((newer_index, item))
}
