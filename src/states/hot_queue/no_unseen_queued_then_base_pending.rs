use std::task::{Poll, Waker};

use futures::Stream;

use super::{
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
    unseen_queued_item_ready::UnseenQueuedItemReady,
};
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult, StateHandler, common::poll_base_stream},
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBasePending {
    pub(crate) waker: Waker,
    pub(crate) most_recent_queue_item_index: usize,
}

impl std::fmt::Debug for NoUnseenQueuedThenBasePending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoUnseenQueuedThenBasePending")
            .field(
                "most_recent_queue_item_index",
                &self.most_recent_queue_item_index,
            )
            .finish_non_exhaustive()
    }
}

impl StateHandler for NoUnseenQueuedThenBasePending {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match fork.queue.keys().copied().find(|queue_index| {
            fork.queue
                .is_strictly_newer_then(*queue_index, self.most_recent_queue_item_index)
        }) {
            Some(newer_queue_item_index) => {
                let item = fork.queue.get(newer_queue_item_index).unwrap().clone();
                if !fork.clones.iter().any(|(clone_id, _state)| {
                    fork.clone_should_still_see_item(*clone_id, newer_queue_item_index)
                }) {
                    fork.queue.remove(newer_queue_item_index);
                }

                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: newer_queue_item_index,
                    }),
                    poll_result: Poll::Ready(item.clone()),
                }
            }
            None => match poll_base_stream(clone_id, waker, fork) {
                Poll::Ready(item) => NewStateAndPollResult {
                    new_state: CloneState::NoUnseenQueuedThenBaseReady(NoUnseenQueuedThenBaseReady),
                    poll_result: Poll::Ready(item),
                },
                Poll::Pending => NewStateAndPollResult {
                    new_state: CloneState::NoUnseenQueuedThenBasePending(
                        NoUnseenQueuedThenBasePending {
                            waker: waker.clone(),
                            most_recent_queue_item_index: self.most_recent_queue_item_index,
                        },
                    ),
                    poll_result: Poll::Pending,
                },
            },
        }
    }
}
