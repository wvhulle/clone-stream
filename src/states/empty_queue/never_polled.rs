use std::task::{Context, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::{
    Fork,
    states::{
        NewStateAndPollResult, StateHandler, transitions,
    },
};

#[derive(Default, Clone, Debug)]
pub(crate) struct NeverPolled;

impl StateHandler for NeverPolled {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        trace!("This clone has never been polled before.");
        match fork
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
        {
            std::task::Poll::Ready(item) => {
                trace!("The base stream is ready.");
                if fork.has_other_clones_waiting(clone_id) {
                    trace!("At least one clone is interested in the new item.");
                    fork.queue.insert(item.clone());
                } else {
                    trace!("No other clone is interested in the new item.");
                }

                NewStateAndPollResult::ready(transitions::to_queue_empty_ready(), item)
            }
            std::task::Poll::Pending => {
                trace!("The base stream is pending.");
                let new_state = if fork.queue.is_empty() {
                    trace!("The item queue is empty.");
                    transitions::to_queue_empty_pending(waker)
                } else {
                    trace!("The item queue is not empty.");
                    transitions::to_no_unseen_pending(waker, fork.queue.newest_index().unwrap())
                };
                NewStateAndPollResult::pending(new_state)
            }
        }
    }
}
