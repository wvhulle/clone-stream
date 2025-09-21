use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use super::{
    no_unseen_queued_then_base_pending::NoUnseenQueuedThenBasePending,
    no_unseen_queued_then_base_ready::NoUnseenQueuedThenBaseReady,
};
use crate::{
    Fork,
    states::{CloneState, NewStateAndPollResult, StateHandler},
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
                trace!("Clone {clone_id}: UnseenQueuedItemReady advancing from index {} to {}", self.unseen_ready_queue_item_index, newer_queue_item_index);

                NewStateAndPollResult {
                    new_state: CloneState::UnseenQueuedItemReady(UnseenQueuedItemReady {
                        unseen_ready_queue_item_index: newer_queue_item_index,
                    }),
                    poll_result: Poll::Ready(item),
                }
            }
            None => {
                match fork
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
                {
                    Poll::Ready(item) => {
                        if fork.has_other_clones_waiting(clone_id) {
                            fork.queue.insert(item.clone());
                        }
                        // If allocation fails, we continue without queuing the item
                        trace!("Clone {clone_id}: UnseenQueuedItemReady transitioning to NoUnseenQueuedThenBaseReady with new base item");
                        NewStateAndPollResult {
                            new_state: CloneState::NoUnseenQueuedThenBaseReady(
                                NoUnseenQueuedThenBaseReady,
                            ),
                            poll_result: Poll::Ready(item.clone()),
                        }
                    }
                    Poll::Pending => NewStateAndPollResult {
                        new_state: CloneState::NoUnseenQueuedThenBasePending(
                            NoUnseenQueuedThenBasePending {
                                waker: waker.clone(),
                                most_recent_queue_item_index: self.unseen_ready_queue_item_index,
                            },
                        ),
                        poll_result: Poll::Pending,
                    },
                }
            }
        }
    }
}
