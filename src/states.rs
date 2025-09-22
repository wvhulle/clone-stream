use std::{
    fmt::Debug,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::{debug, trace};

use crate::Fork;

/// Represents the state of a clone in the stream cloning state machine.
///
/// Each clone maintains its own state to track its position relative to the base stream
/// and the shared queue. The state determines how the clone should behave when polled.
#[derive(Clone, Debug)]
pub(crate) enum CloneState {
    /// Initial state when a clone is first created but has never been polled.
    ///
    /// When polled, the clone will attempt to read from the base stream. If successful,
    /// it transitions to `QueueEmpty`. If the base stream is pending, it transitions
    /// to either `QueueEmptyPending` or `AllSeenPending` depending on queue contents.
    Initial,

    /// The queue is empty and the clone can read directly from the base stream.
    ///
    /// This state indicates that there are no queued items and the clone is ready
    /// to poll the base stream immediately. After polling, it either gets an item
    /// (staying in this state) or transitions to a pending state.
    QueueEmpty,

    /// The queue is empty but the base stream is currently pending.
    ///
    /// The clone is waiting for the base stream to produce an item. Contains a waker
    /// that will be notified when the base stream has new data available.
    QueueEmptyPending {
        /// Waker to notify this clone when the base stream becomes ready
        waker: Waker,
    },

    /// All queued items have been seen by this clone, but the base stream is pending.
    ///
    /// The clone has processed all available queue items up to `last_seen_index`
    /// and is now waiting for either new queue items or the base stream to produce
    /// new data. Contains the index of the most recent item this clone has seen.
    AllSeenPending {
        /// Waker to notify this clone when new data becomes available
        waker: Waker,
        /// Index of the last queue item this clone has seen
        last_seen_index: usize,
    },

    /// All queued items have been seen and the clone can read from the base stream.
    ///
    /// Similar to `QueueEmpty`, but this clone has previously seen queued items.
    /// The clone is ready to poll the base stream for new items.
    AllSeen,

    /// There is an unseen queued item ready for this clone to consume.
    ///
    /// The clone has identified a specific queue item at `unseen_index` that it
    /// needs to process. When polled, it will return this item and potentially
    /// advance to the next unseen item or transition to another state.
    UnseenReady {
        /// Index of the queue item that this clone should process next
        unseen_index: usize,
    },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::Initial
    }
}

impl CloneState {
    pub(crate) fn queue_empty_pending(waker: &Waker) -> Self {
        Self::QueueEmptyPending {
            waker: waker.clone(),
        }
    }

    pub(crate) fn queue_empty() -> Self {
        Self::QueueEmpty
    }

    pub(crate) fn all_seen_pending(waker: &Waker, index: usize) -> Self {
        Self::AllSeenPending {
            waker: waker.clone(),
            last_seen_index: index,
        }
    }

    pub(crate) fn all_seen() -> Self {
        Self::AllSeen
    }

    pub(crate) fn unseen_ready(index: usize) -> Self {
        Self::UnseenReady {
            unseen_index: index,
        }
    }

    pub(crate) fn should_still_see_base_item(&self) -> bool {
        match self {
            CloneState::Initial
            | CloneState::QueueEmptyPending { .. }
            | CloneState::AllSeenPending { .. } => true,
            CloneState::QueueEmpty | CloneState::AllSeen | CloneState::UnseenReady { .. } => false,
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
            | CloneState::UnseenReady { .. } => None,
        }
    }
}

impl CloneState {
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
                transition_return(
                    self,
                    poll_base_with_queue_check(clone_id, waker, fork, true),
                    CloneState::queue_empty(),
                    next_pending_state(waker, fork),
                )
            }
            CloneState::QueueEmpty => {
                debug!("Clone {clone_id}: Queue empty, polling base stream");
                transition_return(
                    self,
                    poll_base_with_queue_check(clone_id, waker, fork, false),
                    CloneState::queue_empty(),
                    next_pending_state(waker, fork),
                )
            }
            CloneState::QueueEmptyPending { .. } => {
                if fork.queue.is_empty() {
                    debug!("Clone {clone_id}: Queue still empty, polling base stream");
                    transition_return(
                        self,
                        poll_base_with_queue_check(clone_id, waker, fork, true),
                        CloneState::queue_empty(),
                        CloneState::queue_empty_pending(waker),
                    )
                } else {
                    debug!("Clone {clone_id}: Queue now has items");
                    let Some(first_queue_index) = fork.queue.oldest else {
                        debug!("Clone {clone_id}: Queue oldest index unavailable, staying pending");
                        *self = CloneState::queue_empty_pending(waker);
                        return Poll::Pending;
                    };
                    trace!("Clone {clone_id}: Processing queue item at index {first_queue_index}");

                    let item = process_oldest_queue_item(fork, clone_id, first_queue_index);
                    debug!("Clone {clone_id}: Transitioning to UnseenReady at index {first_queue_index}");
                    *self = CloneState::unseen_ready(first_queue_index);
                    Poll::Ready(item)
                }
            }
            CloneState::AllSeenPending {
                last_seen_index, ..
            } => {
                let last_seen = *last_seen_index;
                if let Some((newer_index, item)) = process_newer_queue_item(fork, last_seen, true) {
                    debug!("Clone {clone_id}: Found newer queue item at index {newer_index}");
                    *self = CloneState::unseen_ready(newer_index);
                    Poll::Ready(item)
                } else {
                    debug!("Clone {clone_id}: No newer queue items, polling base stream");
                    transition_return(
                        self,
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::all_seen(),
                        CloneState::all_seen_pending(waker, last_seen),
                    )
                }
            }
            CloneState::AllSeen => {
                debug!("Clone {clone_id}: All seen, polling base stream");
                let pending_state = if let Some(oldest_index) = fork.queue.oldest {
                    CloneState::all_seen_pending(waker, oldest_index)
                } else {
                    CloneState::queue_empty_pending(waker)
                };
                transition_return(
                    self,
                    poll_base_stream(clone_id, waker, fork),
                    CloneState::all_seen(),
                    pending_state,
                )
            }
            CloneState::UnseenReady { unseen_index } => {
                let unseen = *unseen_index;
                if let Some((newer_index, item)) = process_newer_queue_item(fork, unseen, false) {
                    debug!("Clone {clone_id}: Advancing from index {unseen} to {newer_index}");
                    *self = CloneState::unseen_ready(newer_index);
                    Poll::Ready(item)
                } else {
                    debug!("Clone {clone_id}: No more unseen items, transitioning to AllSeen");
                    transition_return(
                        self,
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::all_seen(),
                        CloneState::all_seen_pending(waker, unseen),
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
    use_other_clones_check: bool,
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
            let should_queue = if use_other_clones_check {
                fork.has_other_clones_waiting(clone_id)
            } else {
                fork.clones.iter().any(|(other_clone_id, state)| {
                    *other_clone_id != clone_id && state.should_still_see_base_item()
                })
            };

            if should_queue {
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

fn find_newer_queue_item<BaseStream>(fork: &Fork<BaseStream>, current_index: usize) -> Option<usize>
where
    BaseStream: Stream<Item: Clone>,
{
    fork.queue
        .keys()
        .copied()
        .find(|queue_index| fork.queue.is_newer_than(*queue_index, current_index))
}

fn next_pending_state<BaseStream>(waker: &Waker, fork: &Fork<BaseStream>) -> CloneState
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.queue.is_empty() {
        CloneState::queue_empty_pending(waker)
    } else if let Some(newest_index) = fork.queue.newest {
        CloneState::all_seen_pending(waker, newest_index)
    } else {
        CloneState::queue_empty_pending(waker)
    }
}

fn process_oldest_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    clone_id: usize,
    first_queue_index: usize,
) -> Option<BaseStream::Item>
where
    BaseStream: Stream<Item: Clone>,
{
    let clones_waiting: Vec<_> = fork
        .clones
        .iter()
        .filter(|(other_clone_id, _state)| {
            fork.clone_should_still_see_item(**other_clone_id, first_queue_index)
        })
        .map(|(clone_id, _state)| clone_id)
        .collect();

    if clones_waiting.is_empty() {
        trace!("Clone {clone_id}: Popping queue item at index {first_queue_index} (no other clones waiting)");
        fork.queue.pop_oldest().unwrap().1
    } else {
        trace!("Clone {clone_id}: Cloning queue item at index {first_queue_index} (clones {clones_waiting:?} also waiting)");
        fork.queue.get(first_queue_index).unwrap().clone()
    }
}

fn transition_return<Item>(
    state: &mut CloneState,
    poll_result: Poll<Option<Item>>,
    ready_state: CloneState,
    pending_state: CloneState,
) -> Poll<Option<Item>> {
    let original_state = format!("{state:?}");
    match poll_result {
        Poll::Ready(item) => {
            debug!("State transition: {original_state} -> {ready_state:?}");
            *state = ready_state;
            Poll::Ready(item)
        }
        Poll::Pending => {
            debug!("State transition: {original_state} -> {pending_state:?}");
            *state = pending_state;
            Poll::Pending
        }
    }
}

fn process_newer_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    current_index: usize,
    should_cleanup: bool,
) -> Option<(usize, Option<BaseStream::Item>)>
where
    BaseStream: Stream<Item: Clone>,
{
    find_newer_queue_item(fork, current_index).map(|newer_index| {
        let item = fork.queue.get(newer_index).unwrap().clone();

        if should_cleanup
            && !fork
                .clones
                .iter()
                .any(|(clone_id, _state)| fork.clone_should_still_see_item(*clone_id, newer_index))
        {
            fork.queue.remove(newer_index);
        }

        (newer_index, item)
    })
}
