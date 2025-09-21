use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};

use super::{
    no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending,
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
};
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult, StateHandler},
};

#[derive(Clone, Debug)]
pub(crate) struct UnseenQueuedItemReady {
    pub(crate) unseen_ready_queue_item_index: usize,
}

impl StateHandler for UnseenQueuedItemReady {
    fn handle<BaseStream>(
        &self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match fork.queue.keys().copied().find(|queue_index| {
            fork.queue
                .is_after(*queue_index, self.unseen_ready_queue_item_index)
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
                    poll_result: Poll::Ready(item),
                }
            }
            None => {
                match fork
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
                {
                    Poll::Ready(item) => {
                        if fork
                            .clones
                            .iter()
                            .any(|(_clone_id, state)| state.should_still_see_base_item())
                        {
                            fork.queue.insert(item.clone());
                        }
                        // If allocation fails, we continue without queuing the item
                        NewStateAndPollResult {
                            new_state: CloneState::NoUnseenQueuedThenBaseReady(
                                NoUnseenQueuedThenBaseReady,
                            ),
                            poll_result: Poll::Ready(item.clone()),
                        }
                    }
                    Poll::Pending => NewStateAndPollResult {
                        new_state: CloneState::NoUnseenQueuedThenBasePending(
                            NoUnseenQueuedThenBasePending {
                                waker: waker.clone(),
                                most_recent_queue_item_index: self.unseen_ready_queue_item_index,
                            },
                        ),
                        poll_result: Poll::Pending,
                    },
                }
            }
        }
    }
}
