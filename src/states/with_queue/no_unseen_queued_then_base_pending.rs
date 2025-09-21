use std::task::{Poll, Waker};

use futures::Stream;

use crate::{
    Fork,
    states::{NewStateAndPollResult, StateHandler, poll_base_stream, transitions},
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBasePending {
    pub(crate) waker: Waker,
    pub(crate) most_recent_queue_item_index: usize,
}

impl std::fmt::Debug for NoUnseenQueuedThenBasePending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoUnseenQueuedThenBasePending")
            .field("most_recent_queue_item_index", &self.most_recent_queue_item_index)
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

                NewStateAndPollResult::ready(
                    transitions::to_unseen_item_ready(newer_queue_item_index),
                    item.clone(),
                )
            }
            None => match poll_base_stream(clone_id, waker, fork) {
                Poll::Ready(item) => NewStateAndPollResult::ready(transitions::to_no_unseen_ready(), item),
                Poll::Pending => NewStateAndPollResult::pending(
                    transitions::to_no_unseen_pending(waker, self.most_recent_queue_item_index)
                ),
            },
        }
    }
}
