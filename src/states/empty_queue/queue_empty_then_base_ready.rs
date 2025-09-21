use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::{
    Fork,
    states::{NewStateAndPollResult, StateHandler, transitions},
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
                if fork.clones.iter().any(|(other_clone_id, state)| {
                    *other_clone_id != clone_id && state.should_still_see_base_item()
                }) {
                    trace!("Other clones are waiting for the new item.");
                    fork.queue.push(item.clone());
                    // If allocation fails, we continue without queuing the item
                } else {
                    trace!("No other clone is interested in the new item.");
                }
                trace!("Clone {clone_id}: QueueEmptyThenBaseReady returning item from base stream");
                NewStateAndPollResult::ready(transitions::to_queue_empty_ready(), item)
            }
            Poll::Pending => {
                // If queue has items, this clone has already seen them (since it was in
                // QueueEmptyThenBaseReady) so transition to
                // NoUnseenQueuedThenBasePending with the most recent queue index
                if let Some(newest_index) = fork.queue.newest {
                    NewStateAndPollResult::pending(transitions::to_no_unseen_pending(waker, newest_index))
                } else {
                    NewStateAndPollResult::pending(transitions::to_queue_empty_pending(waker))
                }
            }
        }
    }
}
