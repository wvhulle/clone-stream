use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use super::queue_empty_then_base_ready::QueueEmptyThenBaseReady;
use crate::{
    Fork,
    states::{
        CloneState, NewStateAndPollResult, StateHandler,
        hot_queue::unseen_queued_item_ready::UnseenQueuedItemReady,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct QueueEmptyThenBasePending {
    pub(crate) waker: Waker,
}

impl StateHandler for QueueEmptyThenBasePending {
    fn handle<BaseStream>(
        self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        trace!("Currently in state 'QueueEmptyThenBasePending");
        if fork.queue.is_empty() {
            trace!("Queue is empty");
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
                        trace!("At least one clone is interested in the new item.");
                        if let Ok(queue_index) = fork.allocate_queue_index() {
                            fork.queue.insert(queue_index, item.clone());
                        }
                        // If allocation fails, we continue without queuing the item
                    } else {
                        trace!("No other clone is interested in the new item.");
                    }
                    NewStateAndPollResult {
                        new_state: CloneState::QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
                        poll_result: Poll::Ready(item),
                    }
                }
                Poll::Pending => NewStateAndPollResult {
                    new_state: CloneState::QueueEmptyThenBasePending(QueueEmptyThenBasePending {
                        waker: waker.clone(),
                    }),
                    poll_result: Poll::Pending,
                },
            }
        } else {
            let first_queue_index = *fork.queue.first_key_value().unwrap().0;
            if fork
                .clones
                .iter()
                .any(|(_clone_id, state)| state.should_still_see_item(first_queue_index))
            {
                trace!("Cloning the first item in the queue.");
                let cloned_item = fork.queue.first_key_value().unwrap().1.clone();
                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: first_queue_index,
                    }),
                    poll_result: Poll::Ready(cloned_item),
                }
            } else {
                trace!("No other clone is waiting for the first item in the queue.");
                let popped_item = fork.queue.pop_first().unwrap().1;
                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: first_queue_index,
                    }),
                    poll_result: Poll::Ready(popped_item),
                }
            }
        }
    }
}
