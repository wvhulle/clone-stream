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
    /// Clone should poll the base stream and may see new items directly.
    ///
    /// This state indicates the clone is either waiting for the base stream,
    /// ready to poll it, or has queue history but should still check the base stream first.
    /// The clone will receive items directly from the base stream when available.
    ///
    /// Fields:
    /// - `waker`: Present when waiting for the base stream to become ready
    /// - `last_seen_index`: Present when clone has seen queue items before
    PollingBaseStream {
        waker: Option<Waker>,
        last_seen_index: Option<usize>,
    },
    /// Clone should process items from the shared queue and avoid the base stream.
    ///
    /// This state indicates the clone is either processing queue items or in an initial
    /// state before seeing any items. The clone will not receive new items directly 
    /// from the base stream in this state and never waits for the base stream.
    ///
    /// Fields:
    /// - `last_seen_index`: Present when clone is processing queue items, None for initial state
    ProcessingQueue { last_seen_index: Option<usize> },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::ProcessingQueue {
            last_seen_index: None,
        }
    }
}

use CloneState::{PollingBaseStream, ProcessingQueue};

impl CloneState {
    #[inline]
    pub(crate) fn should_still_see_base_item(&self) -> bool {
        trace!("Checking if clone in state {self:?} should still see base item");
        matches!(self, PollingBaseStream { .. })
    }
    #[inline]
    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            PollingBaseStream { waker, .. } => waker.clone(),
            ProcessingQueue { .. } => None,
        }
    }
    #[inline]
    fn should_see_with_waker(waker: Waker, last_seen_index: Option<usize>) -> Self {
        PollingBaseStream {
            waker: Some(waker),
            last_seen_index,
        }
    }
    #[inline]
    fn should_not_see_with_index(last_seen_index: usize) -> Self {
        ProcessingQueue {
            last_seen_index: Some(last_seen_index),
        }
    }

    #[inline]
    fn should_see_ready() -> Self {
        PollingBaseStream {
            waker: None,
            last_seen_index: None,
        }
    }
    #[inline]
    fn should_not_see_ready() -> Self {
        ProcessingQueue {
            last_seen_index: None,
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
            PollingBaseStream {
                waker: state_waker,
                last_seen_index,
            } => {
                if let Some(last_seen_index) = last_seen_index {
                    debug!("Clone {clone_id}: has queue history, checking for newer items");
                    let last_seen_index = *last_seen_index;
                    if let Some((newer_index, item)) =
                        process_newer_queue_item(fork, last_seen_index)
                    {
                        *self = Self::should_not_see_with_index(newer_index);
                        return Poll::Ready(item);
                    }

                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        Self::should_not_see_ready(),
                        Self::should_see_with_waker(waker.clone(), Some(last_seen_index)),
                    )
                } else if state_waker.is_some() {
                    debug!("Clone {clone_id}: waiting for base stream");
                    if fork.item_buffer.is_empty() {
                        debug!("Clone {clone_id}: Queue still empty, polling base stream");
                        self.transition_on_poll(
                            poll_base_stream(clone_id, waker, fork),
                            Self::should_see_ready(),
                            Self::should_see_with_waker(waker.clone(), None),
                        )
                    } else {
                        debug!("Clone {clone_id}: Queue now has items, processing oldest");
                        let (oldest_queue_index, item) =
                            pop_or_clone_oldest_unseen_queue_item(fork, clone_id);
                        *self = Self::should_not_see_with_index(oldest_queue_index);
                        Poll::Ready(item)
                    }
                } else {
                    debug!("Clone {clone_id}: ready to poll base stream");
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        Self::should_see_ready(),
                        next_pending_state(waker, fork),
                    )
                }
            }
            ProcessingQueue {
                last_seen_index, ..
            } => {
                if let Some(last_seen_index) = last_seen_index {
                    debug!("Clone {clone_id}: processing queue items");
                    let last_seen_index = *last_seen_index;
                    if let Some((newer_index, item)) =
                        process_newer_queue_item(fork, last_seen_index)
                    {
                        trace!("Clone {clone_id}: Found newer item at {newer_index}");
                        *self = Self::should_not_see_with_index(newer_index);
                        return Poll::Ready(item);
                    }

                    debug!("Clone {clone_id}: No newer queue items, falling back to base stream");
                    let pending_state =
                        Self::should_see_with_waker(waker.clone(), fork.item_buffer.oldest_index());

                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        Self::should_not_see_ready(),
                        pending_state,
                    )
                } else {
                    debug!("Clone {clone_id}: initial state, polling base stream");
                    self.transition_on_poll(
                        poll_base_stream(clone_id, waker, fork),
                        Self::should_see_ready(),
                        next_pending_state(waker, fork),
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
            if fork.clone_registry.has_other_clones_waiting(clone_id) {
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
fn next_pending_state<BaseStream>(waker: &Waker, fork: &Fork<BaseStream>) -> CloneState
where
    BaseStream: Stream<Item: Clone>,
{
    let last_seen_index = if fork.item_buffer.is_empty() {
        None
    } else {
        fork.item_buffer.newest
    };
    CloneState::should_see_with_waker(waker.clone(), last_seen_index)
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

    let item = if fork.clone_registry.count() <= 1 {
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
