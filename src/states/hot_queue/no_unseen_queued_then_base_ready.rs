use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use super::no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending;
use crate::{
    Fork,
    states::{
        CloneState, NewStateAndPollResult, StateHandler,
        cold_queue::queue_empty_then_base_pending::QueueEmptyThenBasePending,
    },
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBaseReady;

impl std::fmt::Debug for NoUnseenQueuedThenBaseReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoUnseenQueuedThenBaseReady").finish()
    }
}

impl StateHandler for NoUnseenQueuedThenBaseReady {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match fork
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
        {
            Poll::Ready(item) => {
                let waiting_clones: Vec<_> = fork
                    .clones
                    .iter()
                    .filter(|(other_clone_id, state)| {
                        **other_clone_id != clone_id && state.should_still_see_base_item()
                    })
                    .map(|(clone_id, _state)| clone_id)
                    .collect();
                if !waiting_clones.is_empty() {
                    trace!("Clones {waiting_clones:?} are waiting for the new item.");
                    fork.queue.insert(item.clone());
                }
                // If allocation fails, we continue without queuing the item
                NewStateAndPollResult {
                    new_state: CloneState::NoUnseenQueuedThenBaseReady(NoUnseenQueuedThenBaseReady),
                    poll_result: Poll::Ready(item),
                }
            }
            Poll::Pending => {
                if fork.queue.is_empty() {
                    NewStateAndPollResult {
                        new_state: CloneState::QueueEmptyThenBasePending(
                            QueueEmptyThenBasePending {
                                waker: waker.clone(),
                            },
                        ),
                        poll_result: Poll::Pending,
                    }
                } else {
                    if let Some(oldest_index) = fork.queue.oldest {
                        NewStateAndPollResult {
                            new_state: CloneState::NoUnseenQueuedThenBasePending(
                                NoUnseenQueuedThenBasePending {
                                    most_recent_queue_item_index: oldest_index,
                                    waker: waker.clone(),
                                },
                            ),
                            poll_result: Poll::Pending,
                        }
                    } else {
                        // Queue has items but oldest is None - this shouldn't happen
                        // Fall back to empty queue state
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
            }
        }
    }
}
