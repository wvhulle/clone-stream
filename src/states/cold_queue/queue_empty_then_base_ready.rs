use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use super::queue_empty_then_base_pending::QueueEmptyThenBasePending;
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult, StateHandler},
};

#[derive(Clone, Debug)]
pub(crate) struct QueueEmptyThenBaseReady;

impl StateHandler for QueueEmptyThenBaseReady {
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
                if waiting_clones.is_empty() {
                    trace!("No other clone is interested in the new item.");
                } else {
                    trace!("Clones {waiting_clones:?} are waiting for the new item.");

                    fork.queue.insert(item.clone());
                    // If allocation fails, we continue without queuing the item
                }
                trace!("Clone {clone_id}: QueueEmptyThenBaseReady returning item from base stream");
                NewStateAndPollResult {
                    new_state: CloneState::QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
                    poll_result: Poll::Ready(item),
                }
            }
            Poll::Pending => {
                // If queue has items, this clone has already seen them (since it was in QueueEmptyThenBaseReady)
                // so transition to NoUnseenQueuedThenBasePending with the most recent queue index
                if let Some(newest_index) = fork.queue.newest {
                    NewStateAndPollResult {
                        new_state: CloneState::NoUnseenQueuedThenBasePending(
                            crate::states::hot_queue::no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending {
                                waker: waker.clone(),
                                most_recent_queue_item_index: newest_index,
                            },
                        ),
                        poll_result: Poll::Pending,
                    }
                } else {
                    NewStateAndPollResult {
                        new_state: CloneState::QueueEmptyThenBasePending(QueueEmptyThenBasePending {
                            waker: waker.clone(),
                        }),
                        poll_result: Poll::Pending,
                    }
                }
            },
        }
    }
}
