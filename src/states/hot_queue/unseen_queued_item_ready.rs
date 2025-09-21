use std::task::{Poll, Waker};

use futures::Stream;
use log::trace;

use crate::{
    Fork,
    states::{NewStateAndPollResult, StateHandler, poll_base_stream, transitions},
};

#[derive(Clone, Debug)]
pub(crate) struct UnseenQueuedItemReady {
    pub(crate) unseen_ready_queue_item_index: usize,
}

impl StateHandler for UnseenQueuedItemReady {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match fork.queue.keys().copied().find(|queue_index| {
            fork.queue
                .is_strictly_newer_then(*queue_index, self.unseen_ready_queue_item_index)
        }) {
            Some(newer_queue_item_index) => {
                let item = fork.queue.get(newer_queue_item_index).unwrap().clone();
                // Let cleanup handle item removal during clone unregistration
                trace!(
                    "Clone {clone_id}: UnseenQueuedItemReady advancing from index {} to {}",
                    self.unseen_ready_queue_item_index, newer_queue_item_index
                );

                NewStateAndPollResult::ready(
                    transitions::to_unseen_item_ready(newer_queue_item_index),
                    item,
                )
            }
            None => match poll_base_stream(clone_id, waker, fork) {
                Poll::Ready(item) => {
                    trace!(
                        "Clone {clone_id}: UnseenQueuedItemReady transitioning to \
                         NoUnseenQueuedThenBaseReady with new base item"
                    );
                    NewStateAndPollResult::ready(transitions::to_no_unseen_ready(), item.clone())
                }
                Poll::Pending => NewStateAndPollResult::pending(
                    transitions::to_no_unseen_pending(waker, self.unseen_ready_queue_item_index)
                ),
            },
        }
    }
}
