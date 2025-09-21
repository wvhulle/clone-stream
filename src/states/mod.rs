use std::{
    fmt::Debug,
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

pub mod cold_queue;
pub mod hot_queue;

use futures::Stream;

use crate::Fork;

/// Trait for handling state transitions in the clone stream state machine
pub(crate) trait StateHandler {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>;
}

#[derive(Debug, Clone)]
pub(crate) struct NewStateAndPollResult<T> {
    pub(crate) new_state: CloneState,
    pub(crate) poll_result: Poll<T>,
}

#[derive(Clone, Debug)]
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
    pub(crate) fn should_still_see_base_item(&self) -> bool {
        match self {
            CloneState::NeverPolled(_)
            | CloneState::QueueEmptyThenBasePending(_)
            | CloneState::NoUnseenQueuedThenBasePending(_) => true,
            CloneState::QueueEmptyThenBaseReady(_)
            | CloneState::NoUnseenQueuedThenBaseReady(_)
            | CloneState::UnseenQueuedItemReady(_) => false,
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            CloneState::QueueEmptyThenBasePending(queue_empty_then_base_pending) => {
                Some(queue_empty_then_base_pending.waker.clone())
            }
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                Some(no_unseen_queued_then_base_pending.waker.clone())
            }
            CloneState::NeverPolled(_)
            | CloneState::QueueEmptyThenBaseReady(_)
            | CloneState::NoUnseenQueuedThenBaseReady(_)
            | CloneState::UnseenQueuedItemReady(_) => None,
        }
    }
}

impl StateHandler for CloneState {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match self {
            CloneState::NeverPolled(state) => state.handle(clone_id, waker, fork),
            CloneState::QueueEmptyThenBaseReady(state) => state.handle(clone_id, waker, fork),
            CloneState::QueueEmptyThenBasePending(state) => state.handle(clone_id, waker, fork),
            CloneState::NoUnseenQueuedThenBasePending(state) => state.handle(clone_id, waker, fork),
            CloneState::NoUnseenQueuedThenBaseReady(state) => state.handle(clone_id, waker, fork),
            CloneState::UnseenQueuedItemReady(state) => state.handle(clone_id, waker, fork),
        }
    }
}
