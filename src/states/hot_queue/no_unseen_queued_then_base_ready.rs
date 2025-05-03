use std::{
    fmt::Display,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

use super::no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending;
use crate::{
    Fork,
    states::{
        CloneState, NewStateAndPollResult,
        cold_queue::queue_empty_then_base_pending::QueueEmptyThenBasePending,
    },
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBaseReady;

impl Display for NoUnseenQueuedThenBaseReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoUnseenQueuedThenBaseReady")
    }
}

impl NoUnseenQueuedThenBaseReady {
    pub(crate) fn handle<BaseStream>(
        self,
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
                if fork
                    .clones
                    .iter()
                    .any(|(_clone_id, state)| state.should_still_see_base_item())
                {
                    fork.queue.insert(fork.next_queue_index, item.clone());
                    fork.next_queue_index += 1;
                    
                }
                NewStateAndPollResult {
                    new_state: CloneState::NoUnseenQueuedThenBaseReady(NoUnseenQueuedThenBaseReady),
                    poll_result: Poll::Ready(item.clone()),
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
                    NewStateAndPollResult {
                        new_state: CloneState::NoUnseenQueuedThenBasePending(
                            NoUnseenQueuedThenBasePending {
                                most_recent_queue_item_index: *fork
                                    .queue
                                    .first_entry()
                                    .unwrap()
                                    .key(),
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
