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
    Initial,
    QueueEmpty,

    QueueEmptyPending {
        waker: Waker,
    },

    AllSeenPending {
        waker: Waker,
        last_seen_index: usize,
    },

    AllSeen,
    PreviouslySawOnQueue {
        last_seen_queue_index: usize,
    },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::Initial
    }
}

impl CloneState {
    pub(crate) fn should_still_see_base_item(&self) -> bool {
        trace!("Checking if clone in state {self:?} should still see base item");
        match self {
            CloneState::QueueEmptyPending { .. }
            | CloneState::AllSeenPending { .. }
            | CloneState::QueueEmpty => true,
            CloneState::Initial | CloneState::AllSeen | CloneState::PreviouslySawOnQueue { .. } => {
                false
            }
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            CloneState::QueueEmptyPending { waker } | CloneState::AllSeenPending { waker, .. } => {
                Some(waker.clone())
            }
            CloneState::Initial
            | CloneState::QueueEmpty
            | CloneState::AllSeen
            | CloneState::PreviouslySawOnQueue { .. } => None,
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
    /// Main state machine step - processes the clone's current state
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
            CloneState::Initial => {
                debug!("Clone {clone_id}: Initial poll");
                self.transition_on_poll(
                    poll_base_with_queue_check(clone_id, waker, fork),
                    CloneState::QueueEmpty,
                    next_pending_state(waker, fork),
                )
            }
            CloneState::QueueEmpty => self.transition_on_poll(
                poll_base_with_queue_check(clone_id, waker, fork),
                CloneState::QueueEmpty,
                next_pending_state(waker, fork),
            ),
            CloneState::QueueEmptyPending { .. } => {
                debug!("Clone {clone_id}: Queue used to be empty, check if it still is");
                if fork.queue.is_empty() {
                    debug!("Clone {clone_id}: Queue still empty, polling base stream");
                    self.transition_on_poll(
                        poll_base_with_queue_check(clone_id, waker, fork),
                        CloneState::QueueEmpty,
                        CloneState::QueueEmptyPending {
                            waker: waker.clone(),
                        },
                    )
                } else {
                    debug!("Clone {clone_id}: Queue now has items, processing oldest");
                    let (oldest_queue_index, item) =
                        pop_or_clone_oldest_unseen_queue_item(fork, clone_id);
                    *self = CloneState::PreviouslySawOnQueue {
                        last_seen_queue_index: oldest_queue_index,
                    };
                    Poll::Ready(item)
                }
            }
            CloneState::AllSeenPending {
                last_seen_index, ..
            } => {
                let last_seen_index = *last_seen_index;
                if let Some((newer_index, item)) = process_newer_queue_item(fork, last_seen_index) {
                    *self = CloneState::PreviouslySawOnQueue {
                        last_seen_queue_index: newer_index,
                    };
                    Poll::Ready(item)
                } else {
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::AllSeen,
                        CloneState::AllSeenPending {
                            waker: waker.clone(),
                            last_seen_index,
                        },
                    )
                }
            }
            CloneState::AllSeen => {
                let pending_state = if let Some(oldest_index) = fork.queue.oldest_index() {
                    CloneState::AllSeenPending {
                        waker: waker.clone(),
                        last_seen_index: oldest_index,
                    }
                } else {
                    CloneState::QueueEmptyPending {
                        waker: waker.clone(),
                    }
                };

                self.transition_on_poll(
                    poll_base_stream(clone_id, waker, fork),
                    CloneState::AllSeen,
                    pending_state,
                )
            }
            CloneState::PreviouslySawOnQueue {
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
                    *self = CloneState::PreviouslySawOnQueue {
                        last_seen_queue_index: newer_index,
                    };
                    Poll::Ready(item)
                } else {
                    trace!("Clone {clone_id}: No newer item, transitioning to AllSeen");
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::AllSeen,
                        CloneState::AllSeenPending {
                            waker: waker.clone(),
                            last_seen_index: last_seen_queue_index,
                        },
                    )
                }
            }
        }
    }
}

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
                fork.queue.push(item.clone());
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
                fork.queue.push(item.clone());
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

fn next_pending_state<BaseStream>(waker: &Waker, fork: &Fork<BaseStream>) -> CloneState
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.queue.is_empty() {
        CloneState::QueueEmptyPending {
            waker: waker.clone(),
        }
    } else if let Some(newest_index) = fork.queue.newest {
        CloneState::AllSeenPending {
            waker: waker.clone(),
            last_seen_index: newest_index,
        }
    } else {
        CloneState::QueueEmptyPending {
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
        .queue
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
        fork.queue
            .get(previous_occupied_oldest_queue_index)
            .unwrap()
            .clone()
    } else {
        fork.queue.pop_oldest().unwrap()
    };

    (previous_occupied_oldest_queue_index, oldest_queue_item)
}

#[inline]
fn process_newer_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    current_index: usize,
) -> Option<(usize, Option<BaseStream::Item>)>
where
    BaseStream: Stream<Item: Clone>,
{
    let newer_index = fork.queue.find_next_newer_index(current_index)?;
    let item = get_queue_item_optimally(fork, newer_index);
    Some((newer_index, item))
}

#[inline]
fn get_queue_item_optimally<BaseStream>(
    fork: &mut Fork<BaseStream>,
    index: usize,
) -> Option<BaseStream::Item>
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.active_clone_count() <= 1 {
        return fork.queue.remove(index).unwrap();
    }

    let clones_needing_item = count_clones_needing_item(fork, index);

    match clones_needing_item {
        0 | 1 => fork.queue.remove(index).unwrap(),
        _ => fork.queue.get(index).unwrap().clone(),
    }
}

#[inline]
fn count_clones_needing_item<BaseStream>(fork: &Fork<BaseStream>, index: usize) -> usize
where
    BaseStream: Stream<Item: Clone>,
{
    fork.clone_registry
        .iter_active_with_ids()
        .filter(|(clone_id, _)| fork.should_clone_see_item(*clone_id, index))
        .count()
}
