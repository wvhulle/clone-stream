use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::{debug, trace};

use crate::{
    Fork,
    states::{NewStateAndPollResult, StateHandler, transitions},
};

#[derive(Clone)]
pub(crate) struct QueueEmptyThenBasePending {
    pub(crate) waker: Waker,
}

impl std::fmt::Debug for QueueEmptyThenBasePending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEmptyThenBasePending")
            .finish_non_exhaustive()
    }
}

impl StateHandler for QueueEmptyThenBasePending {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        if fork.queue.is_empty() {
            trace!("Queue is empty");
            match fork
                .base_stream
                .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
            {
                Poll::Ready(item) => {
                    trace!("The base stream is ready.");
                    if fork.has_other_clones_waiting(clone_id) {
                        trace!("Other clones are interested in the new item.");
                        fork.queue.insert(item.clone());
                        // If allocation fails, we continue without queuing the
                        // item
                    } else {
                        trace!("No other clone is interested in the new item.");
                    }
                    NewStateAndPollResult::ready(transitions::to_queue_empty_ready(), item)
                }
                Poll::Pending => {
                    debug!("The base stream is still pending.");
                    NewStateAndPollResult::pending(transitions::to_queue_empty_pending(waker))
                }
            }
        } else {
            trace!("The queue is not empty.");
            let Some(first_queue_index) = fork.queue.oldest else {
                // Queue has items but oldest is None - fallback to empty queue behavior
                return NewStateAndPollResult::pending(transitions::to_queue_empty_pending(waker));
            };
            trace!("The queue is not empty, first item is at index {first_queue_index}.");

            let clones_waiting: Vec<_> = fork
                .clones
                .iter()
                .filter(|(other_clone_id, _state)| {
                    fork.clone_should_still_see_item(**other_clone_id, first_queue_index)
                })
                .map(|(clone_id, _state)| clone_id)
                .collect();

            if clones_waiting.is_empty() {
                trace!("No other clone is waiting for the first item in the queue.");
                let popped_item = fork.queue.oldest_with_index().unwrap().1;
                trace!(
                    "Clone {clone_id}: QueueEmptyThenBasePending popping item at index \
                     {first_queue_index}"
                );
                NewStateAndPollResult::ready(
                    transitions::to_unseen_item_ready(first_queue_index),
                    popped_item,
                )
            } else {
                trace!("Forks {clones_waiting:?} also need to see the first item in the queue.");
                let cloned_item = fork.queue.get(first_queue_index).unwrap().clone();
                trace!(
                    "Clone {clone_id}: QueueEmptyThenBasePending cloning item at index \
                     {first_queue_index} (other clones waiting)"
                );
                NewStateAndPollResult::ready(
                    transitions::to_unseen_item_ready(first_queue_index),
                    cloned_item,
                )
            }
        }
    }
}
