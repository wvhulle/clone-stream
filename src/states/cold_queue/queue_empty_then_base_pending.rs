use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::{debug, trace};

use super::queue_empty_then_base_ready::QueueEmptyThenBaseReady;
use crate::{
    Fork,
    states::{
        CloneState, NewStateAndPollResult, StateHandler,
        hot_queue::unseen_queued_item_ready::UnseenQueuedItemReady,
    },
};

#[derive(Clone)]
pub(crate) struct QueueEmptyThenBasePending {
    pub(crate) waker: Waker,
}

impl std::fmt::Debug for QueueEmptyThenBasePending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEmptyThenBasePending").finish()
    }
}

impl StateHandler for QueueEmptyThenBasePending {
    fn handle<BaseStream>(
        &self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        if fork.queue.is_empty() {
            trace!("Queue is empty");
            match fork
                .base_stream
                .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
            {
                Poll::Ready(item) => {
                    trace!("The base stream is ready.");
                    let waiting_clones: Vec<_> = fork
                        .clones
                        .iter()
                        .filter(|(_clone_id, state)| state.should_still_see_base_item())
                        .map(|(clone_id, _state)| clone_id)
                        .collect();
                    if waiting_clones.is_empty() {
                        trace!("No other clone is interested in the new item.");
                    } else {
                        trace!("Clones {waiting_clones:?} are interested in the new item.");
                        fork.queue.insert(item.clone());
                        // If allocation fails, we continue without queuing the
                        // item
                    }
                    NewStateAndPollResult {
                        new_state: CloneState::QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
                        poll_result: Poll::Ready(item),
                    }
                }
                Poll::Pending => {
                    debug!("The base stream is still pending.");
                    NewStateAndPollResult {
                        new_state: CloneState::QueueEmptyThenBasePending(
                            QueueEmptyThenBasePending {
                                waker: waker.clone(),
                            },
                        ),
                        poll_result: Poll::Pending,
                    }
                }
            }
        } else {
            trace!("The queue is not empty.");
            let first_queue_index = fork.queue.oldest.unwrap();
            trace!("The queue is not empty, first item is at index {first_queue_index}.");
            let clones_waiting: Vec<_> = fork
                .clones
                .iter()
                .filter(|(clone_id, _state)| {
                    fork.clone_should_still_see_item(**clone_id, first_queue_index)
                })
                .map(|(clone_id, _state)| clone_id)
                .collect();

            if clones_waiting.is_empty() {
                trace!("No other clone is waiting for the first item in the queue.");
                let popped_item = fork.queue.oldest_with_index().unwrap().1;
                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: first_queue_index,
                    }),
                    poll_result: Poll::Ready(popped_item),
                }
            } else {
                trace!("Forks {clones_waiting:?} also need to see the first item in the queue.");
                let cloned_item = fork.queue.oldest_with_index().unwrap().1.clone();
                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: first_queue_index,
                    }),
                    poll_result: Poll::Ready(cloned_item),
                }
            }
        }
    }
}
