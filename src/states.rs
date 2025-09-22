use std::{
    fmt::Debug,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

use crate::Fork;

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
            trace!("The base stream is ready.");
            let should_queue = if use_other_clones_check {
                fork.has_other_clones_waiting(clone_id)
            } else {
                fork.clones.iter().any(|(other_clone_id, state)| {
                    *other_clone_id != clone_id && state.should_still_see_base_item()
                })
            };

            if should_queue {
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
        CloneState::no_unseen_pending(waker, newest_index)
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
        trace!("No other clone is waiting for the first item in the queue.");
        let popped_item = fork.queue.pop_oldest().unwrap().1;
        trace!(
            "Clone {clone_id}: QueueEmptyThenBasePending popping item at index \
             {first_queue_index}"
        );
        popped_item
    } else {
        trace!("Forks {clones_waiting:?} also need to see the first item in the queue.");
        let cloned_item = fork.queue.get(first_queue_index).unwrap().clone();
        trace!(
            "Clone {clone_id}: QueueEmptyThenBasePending cloning item at index \
             {first_queue_index} (other clones waiting)"
        );
        cloned_item
    }
}

fn poll_and_transition<Item>(
    state: &mut CloneState,
    poll_result: Poll<Option<Item>>,
    ready_state: CloneState,
    pending_state: CloneState,
) -> Poll<Option<Item>> {
    match poll_result {
        Poll::Ready(item) => {
            *state = ready_state;
            Poll::Ready(item)
        }
        Poll::Pending => {
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
            CloneState::QueueEmptyThenBasePending { waker }
            | CloneState::NoUnseenQueuedThenBasePending { waker, .. } => Some(waker.clone()),
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
                trace!("This clone has never been polled before.");
                poll_and_transition(
                    self,
                    poll_base_with_queue_check(clone_id, waker, fork, true),
                    CloneState::queue_empty_ready(),
                    next_pending_state(waker, fork),
                )
            }
            CloneState::QueueEmptyThenBaseReady => {
                trace!("Clone {clone_id}: QueueEmptyThenBaseReady returning item from base stream");
                poll_and_transition(
                    self,
                    poll_base_with_queue_check(clone_id, waker, fork, false),
                    CloneState::queue_empty_ready(),
                    next_pending_state(waker, fork),
                )
            }
            CloneState::QueueEmptyThenBasePending { .. } => {
                if fork.queue.is_empty() {
                    trace!("Queue is empty");
                    poll_and_transition(
                        self,
                        poll_base_with_queue_check(clone_id, waker, fork, true),
                        CloneState::queue_empty_ready(),
                        CloneState::queue_empty_pending(waker),
                    )
                } else {
                    trace!("The queue is not empty.");
                    let Some(first_queue_index) = fork.queue.oldest else {
                        *self = CloneState::queue_empty_pending(waker);
                        return Poll::Pending;
                    };
                    trace!("The queue is not empty, first item is at index {first_queue_index}.");

                    let item = process_oldest_queue_item(fork, clone_id, first_queue_index);
                    *self = CloneState::unseen_item_ready(first_queue_index);
                    Poll::Ready(item)
                }
            }
            CloneState::NoUnseenQueuedThenBasePending {
                most_recent_queue_item_index,
                ..
            } => {
                let most_recent_index = *most_recent_queue_item_index;
                if let Some((newer_index, item)) =
                    process_newer_queue_item(fork, most_recent_index, true)
                {
                    *self = CloneState::unseen_item_ready(newer_index);
                    Poll::Ready(item)
                } else {
                    poll_and_transition(
                        self,
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::no_unseen_ready(),
                        CloneState::no_unseen_pending(waker, most_recent_index),
                    )
                }
            }
            CloneState::NoUnseenQueuedThenBaseReady => {
                let pending_state = if let Some(oldest_index) = fork.queue.oldest {
                    CloneState::no_unseen_pending(waker, oldest_index)
                } else {
                    CloneState::queue_empty_pending(waker)
                };
                poll_and_transition(
                    self,
                    poll_base_stream(clone_id, waker, fork),
                    CloneState::no_unseen_ready(),
                    pending_state,
                )
            }
            CloneState::UnseenQueuedItemReady {
                unseen_ready_queue_item_index,
            } => {
                let unseen_index = *unseen_ready_queue_item_index;
                if let Some((newer_index, item)) =
                    process_newer_queue_item(fork, unseen_index, false)
                {
                    trace!(
                        "Clone {clone_id}: UnseenQueuedItemReady advancing from index {unseen_index} to {newer_index}"
                    );
                    *self = CloneState::unseen_item_ready(newer_index);
                    Poll::Ready(item)
                } else {
                    trace!(
                        "Clone {clone_id}: UnseenQueuedItemReady transitioning to \
                         NoUnseenQueuedThenBaseReady with new base item"
                    );
                    poll_and_transition(
                        self,
                        poll_base_stream(clone_id, waker, fork),
                        CloneState::no_unseen_ready(),
                        CloneState::no_unseen_pending(waker, unseen_index),
                    )
                }
            }
        }
    }
}
