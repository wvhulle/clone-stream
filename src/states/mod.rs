use std::task::Poll;

use cold_queue::{
    never_polled::NeverPolled, queue_empty_then_base_pending::QueueEmptyThenBasePending,
    queue_empty_then_base_ready::QueueEmptyThenBaseReady,
};
use hot_queue::{
    no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending,
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
    unseen_queued_item_ready::UnseenQueuedItemReady,
};

pub mod cold_queue;
pub mod hot_queue;

#[derive(Debug, Clone)]
pub(crate) struct NewStateAndPollResult<T> {
    pub(crate) new_state: CloneState,
    pub(crate) poll_result: Poll<T>,
}

#[derive(Debug, Clone)]
pub(crate) enum CloneState {
    NeverPolled(NeverPolled),
    QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
    QueueEmptyThenBasePending(QueueEmptyThenBasePending),
    NoUnseenQueuedThenBasePending(NoUnseenQueuedThenBasePending),
    NoUnseenQueuedThenBaseReady(NoUnseenQueuedThenBaseReady),
    UnseenQueuedItemReady(UnseenQueuedItemReady),
}

impl Default for CloneState {
    fn default() -> Self {
        Self::NeverPolled(NeverPolled)
    }
}

impl CloneState {
    pub(crate) fn should_still_see_item(&self, queue_item_index: usize) -> bool {
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

    pub(crate) fn wake_by_ref(&self) {
        match self {
            CloneState::NeverPolled(_never_polled) => {}
            CloneState::QueueEmptyThenBaseReady(_queue_empty_then_base_ready) => {}
            CloneState::QueueEmptyThenBasePending(queue_empty_then_base_pending) => {
                queue_empty_then_base_pending.waker.wake_by_ref();
            }
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                no_unseen_queued_then_base_pending.waker.wake_by_ref();
            }
            CloneState::NoUnseenQueuedThenBaseReady(_no_unseen_queued_then_base_ready) => {}
            CloneState::UnseenQueuedItemReady(_unseen_queued_item_ready) => {}
        }
    }
}
