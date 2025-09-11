use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use super::{
    queue_empty_then_base_pending::QueueEmptyThenBasePending,
    queue_empty_then_base_ready::QueueEmptyThenBaseReady,
};
use crate::{
    Fork,
    states::{
        CloneState, NewStateAndPollResult, StateHandler,
        hot_queue::no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending,
    },
};

#[derive(Default, Clone, Debug)]
pub(crate) struct NeverPolled;

impl StateHandler for NeverPolled {
    fn handle<BaseStream>(
        self,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        trace!("Currently in state 'NeverPolled'");
        match fork
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
        {
            std::task::Poll::Ready(item) => {
                trace!("The base stream is ready.");
                if fork
                    .clones
                    .iter()
                    .any(|(_clone_id, state)| state.should_still_see_base_item())
                {
                    trace!("At least one clone is interested in the new item.");
                    let queue_index = fork.allocate_queue_index();
                    fork.queue.insert(queue_index, item.clone());
                } else {
                    trace!("No other clone is interested in the new item.");
                }

                NewStateAndPollResult {
                    new_state: CloneState::QueueEmptyThenBaseReady(QueueEmptyThenBaseReady),
                    poll_result: Poll::Ready(item),
                }
            }
            std::task::Poll::Pending => NewStateAndPollResult {
                poll_result: Poll::Pending,
                new_state: if fork.queue.is_empty() {
                    CloneState::QueueEmptyThenBasePending(QueueEmptyThenBasePending {
                        waker: waker.clone(),
                    })
                } else {
                    CloneState::NoUnseenQueuedThenBasePending(NoUnseenQueuedThenBasePending {
                        most_recent_queue_item_index: *fork.queue.last_entry().unwrap().key(),
                        waker: waker.clone(),
                    })
                },
            },
        }
    }
}
