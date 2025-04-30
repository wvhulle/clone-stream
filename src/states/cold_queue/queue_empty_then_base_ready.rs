use std::{
    fmt::Display,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

use super::queue_empty_then_base_pending::QueueEmptyThenBasePending;
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult},
};

#[derive(Clone)]
pub(crate) struct QueueEmptyThenBaseReady;

impl Display for QueueEmptyThenBaseReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QueueEmptyThenBaseReady")
    }
}

impl QueueEmptyThenBaseReady {
    pub(crate) fn handle<BaseStream>(
        self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        trace!("The queue was empty on last poll of this clone, but the base was ready.");
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
                    fork.queue.insert(fork.next_queue_index, item.clone());
                    fork.next_queue_index += 1;
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
    }
}
