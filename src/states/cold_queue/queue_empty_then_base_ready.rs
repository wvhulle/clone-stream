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
                    .filter(|(_clone_id, state)| state.should_still_see_base_item())
                    .map(|(clone_id, _state)| clone_id)
                    .collect();
                if waiting_clones.is_empty() {
                    trace!("No other clone is interested in the new item.");
                } else {
                    trace!("Clones {waiting_clones:?} are waiting for the new item.");

                    fork.queue.insert(item.clone());
                    // If allocation fails, we continue without queuing the item
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
    }
}
