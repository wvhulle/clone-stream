use std::{
    fmt::{Debug, Display},
    task::{Poll, Waker},
};

use cold_queue::{
    never_polled::NeverPolled, queue_empty_then_base_pending::QueueEmptyThenBasePending,
    queue_empty_then_base_ready::QueueEmptyThenBaseReady,
};
use hot_queue::{
    no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending,
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
    unseen_queued_item_ready::UnseenQueuedItemReady,
};
use log::trace;

pub mod cold_queue;
pub mod hot_queue;

#[derive(Debug, Clone)]
pub(crate) struct NewStateAndPollResult<T> {
    pub(crate) new_state: CloneState,
    pub(crate) poll_result: Poll<T>,
}

#[derive(Clone)]
pub(crate) enum CloneState {
    NeverPolled(NeverPolled),
    QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
    QueueEmptyThenBasePending(QueueEmptyThenBasePending),
    NoUnseenQueuedThenBasePending(NoUnseenQueuedThenBasePending),
    NoUnseenQueuedThenBaseReady(NoUnseenQueuedThenBaseReady),
    UnseenQueuedItemReady(UnseenQueuedItemReady),
}

impl Display for CloneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneState::NeverPolled(never_polled) => write!(f, "{never_polled}"),
            CloneState::QueueEmptyThenBaseReady(queue_empty_then_base_ready) => {
                write!(f, "{queue_empty_then_base_ready}")
            }
            CloneState::QueueEmptyThenBasePending(queue_empty_then_base_pending) => {
                write!(f, "{queue_empty_then_base_pending}")
            }
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                write!(f, "{no_unseen_queued_then_base_pending}")
            }
            CloneState::NoUnseenQueuedThenBaseReady(no_unseen_queued_then_base_ready) => {
                write!(f, "{no_unseen_queued_then_base_ready}")
            }
            CloneState::UnseenQueuedItemReady(unseen_queued_item_ready) => {
                write!(f, "{unseen_queued_item_ready}")
            }
        }
    }
}

impl Debug for CloneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneState::NeverPolled(never_polled) => write!(f, "{never_polled}"),
            CloneState::QueueEmptyThenBaseReady(queue_empty_then_base_ready) => {
                write!(f, "{queue_empty_then_base_ready}")
            }
            CloneState::QueueEmptyThenBasePending(queue_empty_then_base_pending) => {
                write!(f, "{queue_empty_then_base_pending}")
            }
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                write!(f, "{no_unseen_queued_then_base_pending}")
            }
            CloneState::NoUnseenQueuedThenBaseReady(no_unseen_queued_then_base_ready) => {
                write!(f, "{no_unseen_queued_then_base_ready}")
            }
            CloneState::UnseenQueuedItemReady(unseen_queued_item_ready) => {
                write!(f, "{unseen_queued_item_ready}")
            }
        }
    }
}

impl Default for CloneState {
    fn default() -> Self {
        Self::NeverPolled(NeverPolled)
    }
}

impl CloneState {
    pub(crate) fn should_still_see_item(&self, queue_item_index: usize) -> bool {
        trace!(
            "Checking if state {self} should still see queue item with index {queue_item_index}"
        );
        match self {
            CloneState::NeverPolled(_never_polled) => false,
            CloneState::QueueEmptyThenBaseReady(_queue_empty_then_base_ready) => false,
            CloneState::QueueEmptyThenBasePending(_queue_empty_then_base_pending) => true,
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                no_unseen_queued_then_base_pending.most_recent_queue_item_index < queue_item_index
            }
            CloneState::NoUnseenQueuedThenBaseReady(_no_unseen_queued_then_base_ready) => false,
            CloneState::UnseenQueuedItemReady(_unseen_queued_item_ready) => false,
        }
    }

    pub(crate) fn should_still_see_base_item(&self) -> bool {
        match self {
            CloneState::NeverPolled(_never_polled) => false,
            CloneState::QueueEmptyThenBaseReady(_queue_empty_then_base_ready) => false,
            CloneState::QueueEmptyThenBasePending(_queue_empty_then_base_pending) => true,
            CloneState::NoUnseenQueuedThenBasePending(_no_unseen_queued_then_base_pending) => true,
            CloneState::NoUnseenQueuedThenBaseReady(_no_unseen_queued_then_base_ready) => false,
            CloneState::UnseenQueuedItemReady(_unseen_queued_item_ready) => false,
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            CloneState::NeverPolled(_never_polled) => None,
            CloneState::QueueEmptyThenBaseReady(_queue_empty_then_base_ready) => None,
            CloneState::QueueEmptyThenBasePending(queue_empty_then_base_pending) => {
                Some(queue_empty_then_base_pending.waker.clone())
            }
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                Some(no_unseen_queued_then_base_pending.waker.clone())
            }
            CloneState::NoUnseenQueuedThenBaseReady(_no_unseen_queued_then_base_ready) => None,
            CloneState::UnseenQueuedItemReady(_unseen_queued_item_ready) => None,
        }
    }
}
