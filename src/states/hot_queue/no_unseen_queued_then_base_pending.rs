use std::{
    fmt::Display,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

use super::{
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
    unseen_queued_item_ready::UnseenQueuedItemReady,
};
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult},
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBasePending {
    pub(crate) waker: Waker,
    pub(crate) most_recent_queue_item_index: usize,
}

impl Display for NoUnseenQueuedThenBasePending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoUnseenQueuedThenBasePending")
    }
}

impl NoUnseenQueuedThenBasePending {
    pub(crate) fn handle<BaseStream>(
        self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match fork
            .queue
            .keys()
            .copied()
            .find(|queue_index| *queue_index > self.most_recent_queue_item_index)
        {
            Some(newer_queue_item_index) => {
                let item = fork.queue.get(&newer_queue_item_index).unwrap().clone();
                if !fork
                    .clones
                    .iter()
                    .any(|(_clone_id, state)| state.should_still_see_item(newer_queue_item_index))
                {
                    fork.queue.remove(&newer_queue_item_index);
                }

                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: newer_queue_item_index,
                    }),
                    poll_result: Poll::Ready(item.clone()),
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
                            fork.queue.insert(fork.next_queue_index, item.clone());
                            fork.next_queue_index += 1;
                            fork.wake_sleepers();
                        }
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
                                most_recent_queue_item_index: self.most_recent_queue_item_index,
                            },
                        ),
                        poll_result: Poll::Pending,
                    },
                }
            }
        }
    }
}
