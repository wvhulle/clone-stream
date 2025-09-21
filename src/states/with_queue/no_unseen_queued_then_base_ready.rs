use std::task::{Poll, Waker};

use futures::Stream;

use crate::{
    Fork,
    states::{NewStateAndPollResult, StateHandler, poll_base_stream, transitions},
};

#[derive(Clone)]
pub(crate) struct NoUnseenQueuedThenBaseReady;

impl std::fmt::Debug for NoUnseenQueuedThenBaseReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoUnseenQueuedThenBaseReady").finish()
    }
}

impl StateHandler for NoUnseenQueuedThenBaseReady {
    fn handle<BaseStream>(
        &self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> NewStateAndPollResult<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match poll_base_stream(clone_id, waker, fork) {
            Poll::Ready(item) => {
                NewStateAndPollResult::ready(transitions::to_no_unseen_ready(), item)
            }
            Poll::Pending => {
                if fork.queue.is_empty() {
                    NewStateAndPollResult::pending(transitions::to_queue_empty_pending(waker))
                } else if let Some(oldest_index) = fork.queue.oldest {
                    NewStateAndPollResult::pending(transitions::to_no_unseen_pending(
                        waker,
                        oldest_index,
                    ))
                } else {
                    // Queue has items but oldest is None - this shouldn't happen
                    // Fall back to empty queue state
                    NewStateAndPollResult::pending(transitions::to_queue_empty_pending(waker))
                }
            }
        }
    }
}
