use std::{
    fmt::Debug,
    task::{Context, Poll, Waker},
};


use futures::{Stream, StreamExt};
use log::{debug, trace};

use crate::Fork;


/// Common utility for polling the base stream and inserting items into queue if
/// needed
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
            trace!("The base stream is ready.");
            if fork.has_other_clones_waiting(clone_id) {
                trace!("Other clones are waiting for the new item.");
                fork.queue.push(item.clone());
            } else {
                trace!("No other clone is interested in the new item.");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("The base stream is pending.");
            Poll::Pending
        }
    }
}



#[derive(Clone, Debug)]
pub(crate) enum CloneState {
    NeverPolled,
    QueueEmptyThenBaseReady,
    QueueEmptyThenBasePending {
        waker: Waker,
    },
    NoUnseenQueuedThenBasePending {
        waker: Waker,
        most_recent_queue_item_index: usize,
    },
    NoUnseenQueuedThenBaseReady,
    UnseenQueuedItemReady {
        unseen_ready_queue_item_index: usize,
    },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::NeverPolled
    }
}

impl CloneState {
    pub(crate) fn queue_empty_pending(waker: &Waker) -> Self {
        Self::QueueEmptyThenBasePending {
            waker: waker.clone(),
        }
    }

    pub(crate) fn queue_empty_ready() -> Self {
        Self::QueueEmptyThenBaseReady
    }

    pub(crate) fn no_unseen_pending(waker: &Waker, index: usize) -> Self {
        Self::NoUnseenQueuedThenBasePending {
            waker: waker.clone(),
            most_recent_queue_item_index: index,
        }
    }

    pub(crate) fn no_unseen_ready() -> Self {
        Self::NoUnseenQueuedThenBaseReady
    }

    pub(crate) fn unseen_item_ready(index: usize) -> Self {
        Self::UnseenQueuedItemReady {
            unseen_ready_queue_item_index: index,
        }
    }

    pub(crate) fn should_still_see_base_item(&self) -> bool {
        match self {
            CloneState::NeverPolled
            | CloneState::QueueEmptyThenBasePending { .. }
            | CloneState::NoUnseenQueuedThenBasePending { .. } => true,
            CloneState::QueueEmptyThenBaseReady
            | CloneState::NoUnseenQueuedThenBaseReady
            | CloneState::UnseenQueuedItemReady { .. } => false,
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            CloneState::QueueEmptyThenBasePending { waker } => {
                Some(waker.clone())
            }
            CloneState::NoUnseenQueuedThenBasePending { waker, .. } => {
                Some(waker.clone())
            }
            CloneState::NeverPolled
            | CloneState::QueueEmptyThenBaseReady
            | CloneState::NoUnseenQueuedThenBaseReady
            | CloneState::UnseenQueuedItemReady { .. } => None,
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
            CloneState::NeverPolled => {
                // Never polled logic - moved from never_polled.rs
                trace!("This clone has never been polled before.");
                match fork
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
                {
                    Poll::Ready(item) => {
                        trace!("The base stream is ready.");
                        if fork.has_other_clones_waiting(clone_id) {
                            trace!("At least one clone is interested in the new item.");
                            fork.queue.push(item.clone());
                        } else {
                            trace!("No other clone is interested in the new item.");
                        }

                        *self = CloneState::queue_empty_ready();
                        Poll::Ready(item)
                    }
                    Poll::Pending => {
                        trace!("The base stream is pending.");
                        *self = if fork.queue.is_empty() {
                            trace!("The item queue is empty.");
                            CloneState::queue_empty_pending(waker)
                        } else {
                            trace!("The item queue is not empty.");
                            CloneState::no_unseen_pending(waker, fork.queue.newest.unwrap())
                        };
                        Poll::Pending
                    }
                }
            }
            CloneState::QueueEmptyThenBaseReady => {
                // Queue empty then base ready logic - moved from queue_empty_then_base_ready.rs
                match fork
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
                {
                    Poll::Ready(item) => {
                        if fork.clones.iter().any(|(other_clone_id, state)| {
                            *other_clone_id != clone_id && state.should_still_see_base_item()
                        }) {
                            trace!("Other clones are waiting for the new item.");
                            fork.queue.push(item.clone());
                        } else {
                            trace!("No other clone is interested in the new item.");
                        }
                        trace!("Clone {clone_id}: QueueEmptyThenBaseReady returning item from base stream");
                        *self = CloneState::queue_empty_ready();
                        Poll::Ready(item)
                    }
                    Poll::Pending => {
                        *self = if let Some(newest_index) = fork.queue.newest {
                            CloneState::no_unseen_pending(waker, newest_index)
                        } else {
                            CloneState::queue_empty_pending(waker)
                        };
                        Poll::Pending
                    }
                }
            }
            CloneState::QueueEmptyThenBasePending { .. } => {
                // Queue empty then base pending logic - moved from queue_empty_then_base_pending.rs
                if fork.queue.is_empty() {
                    trace!("Queue is empty");
                    match fork
                        .base_stream
                        .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
                    {
                        Poll::Ready(item) => {
                            trace!("The base stream is ready.");
                            if fork.has_other_clones_waiting(clone_id) {
                                trace!("Other clones are interested in the new item.");
                                fork.queue.push(item.clone());
                            } else {
                                trace!("No other clone is interested in the new item.");
                            }
                            *self = CloneState::queue_empty_ready();
                            Poll::Ready(item)
                        }
                        Poll::Pending => {
                            debug!("The base stream is still pending.");
                            *self = CloneState::queue_empty_pending(waker);
                            Poll::Pending
                        }
                    }
                } else {
                    trace!("The queue is not empty.");
                    let Some(first_queue_index) = fork.queue.oldest else {
                        *self = CloneState::queue_empty_pending(waker);
                        return Poll::Pending;
                    };
                    trace!("The queue is not empty, first item is at index {first_queue_index}.");

                    let clones_waiting: Vec<_> = fork
                        .clones
                        .iter()
                        .filter(|(other_clone_id, _state)| {
                            fork.clone_should_still_see_item(**other_clone_id, first_queue_index)
                        })
                        .map(|(clone_id, _state)| clone_id)
                        .collect();

                    if clones_waiting.is_empty() {
                        trace!("No other clone is waiting for the first item in the queue.");
                        let popped_item = fork.queue.pop_oldest().unwrap().1;
                        trace!(
                            "Clone {clone_id}: QueueEmptyThenBasePending popping item at index \
                             {first_queue_index}"
                        );
                        *self = CloneState::unseen_item_ready(first_queue_index);
                        Poll::Ready(popped_item)
                    } else {
                        trace!("Forks {clones_waiting:?} also need to see the first item in the queue.");
                        let cloned_item = fork.queue.get(first_queue_index).unwrap().clone();
                        trace!(
                            "Clone {clone_id}: QueueEmptyThenBasePending cloning item at index \
                             {first_queue_index} (other clones waiting)"
                        );
                        *self = CloneState::unseen_item_ready(first_queue_index);
                        Poll::Ready(cloned_item)
                    }
                }
            }
            CloneState::NoUnseenQueuedThenBasePending { most_recent_queue_item_index, .. } => {
                // No unseen queued then base pending logic
                match fork.queue.keys().copied().find(|queue_index| {
                    fork.queue
                        .is_newer_than(*queue_index, *most_recent_queue_item_index)
                }) {
                    Some(newer_queue_item_index) => {
                        let item = fork.queue.get(newer_queue_item_index).unwrap().clone();
                        if !fork.clones.iter().any(|(clone_id, _state)| {
                            fork.clone_should_still_see_item(*clone_id, newer_queue_item_index)
                        }) {
                            fork.queue.remove(newer_queue_item_index);
                        }

                        *self = CloneState::unseen_item_ready(newer_queue_item_index);
                        Poll::Ready(item)
                    }
                    None => match poll_base_stream(clone_id, waker, fork) {
                        Poll::Ready(item) => {
                            *self = CloneState::no_unseen_ready();
                            Poll::Ready(item)
                        }
                        Poll::Pending => {
                            *self = CloneState::no_unseen_pending(waker, *most_recent_queue_item_index);
                            Poll::Pending
                        }
                    },
                }
            }
            CloneState::NoUnseenQueuedThenBaseReady => {
                // No unseen queued then base ready logic
                match poll_base_stream(clone_id, waker, fork) {
                    Poll::Ready(item) => {
                        *self = CloneState::no_unseen_ready();
                        Poll::Ready(item)
                    }
                    Poll::Pending => {
                        *self = if fork.queue.is_empty() {
                            CloneState::queue_empty_pending(waker)
                        } else if let Some(oldest_index) = fork.queue.oldest {
                            CloneState::no_unseen_pending(waker, oldest_index)
                        } else {
                            CloneState::queue_empty_pending(waker)
                        };
                        Poll::Pending
                    }
                }
            }
            CloneState::UnseenQueuedItemReady { unseen_ready_queue_item_index } => {
                // Unseen queued item ready logic
                match fork.queue.keys().copied().find(|queue_index| {
                    fork.queue
                        .is_newer_than(*queue_index, *unseen_ready_queue_item_index)
                }) {
                    Some(newer_queue_item_index) => {
                        let item = fork.queue.get(newer_queue_item_index).unwrap().clone();
                        trace!(
                            "Clone {clone_id}: UnseenQueuedItemReady advancing from index {} to {}",
                            *unseen_ready_queue_item_index, newer_queue_item_index
                        );

                        *self = CloneState::unseen_item_ready(newer_queue_item_index);
                        Poll::Ready(item)
                    }
                    None => match poll_base_stream(clone_id, waker, fork) {
                        Poll::Ready(item) => {
                            trace!(
                                "Clone {clone_id}: UnseenQueuedItemReady transitioning to \
                                 NoUnseenQueuedThenBaseReady with new base item"
                            );
                            *self = CloneState::no_unseen_ready();
                            Poll::Ready(item)
                        }
                        Poll::Pending => {
                            *self = CloneState::no_unseen_pending(waker, *unseen_ready_queue_item_index);
                            Poll::Pending
                        }
                    },
                }
            }
        }
    }
}
