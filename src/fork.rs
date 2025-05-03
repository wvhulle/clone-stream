use core::ops::Deref;
use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::Arc,
    task::{Poll, Wake, Waker},
};

use futures::Stream;
use log::trace;

use crate::states::{CloneState, NewStateAndPollResult};

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) base_stream: Pin<Box<BaseStream>>,
    pub(crate) queue: BTreeMap<usize, Option<BaseStream::Item>>,
    pub(crate) clones: BTreeMap<usize, CloneState>,
    next_clone_index: usize,
    pub(crate) next_queue_index: usize,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            clones: BTreeMap::default(),
            queue: BTreeMap::new(),
            next_queue_index: 0,
            next_clone_index: 0,
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled through the fork.");
        let current_state = self.clones.remove(&clone_id).unwrap();

        let NewStateAndPollResult {
            poll_result,
            new_state,
        } = match current_state {
            CloneState::NeverPolled(never_polled) => never_polled.handle(clone_waker, self),
            CloneState::QueueEmptyThenBaseReady(queue_empty_base_pending) => {
                queue_empty_base_pending.handle(clone_waker, self)
            }
            CloneState::QueueEmptyThenBasePending(queue_empty_base_ready) => {
                queue_empty_base_ready.handle(clone_waker, self)
            }
            CloneState::NoUnseenQueuedThenBasePending(ready_from_queue) => {
                ready_from_queue.handle(clone_waker, self)
            }
            CloneState::NoUnseenQueuedThenBaseReady(first_poll_pending_queue_empty) => {
                first_poll_pending_queue_empty.handle(clone_waker, self)
            }
            CloneState::UnseenQueuedItemReady(first_poll_pending_queue_non_empty) => {
                first_poll_pending_queue_non_empty.handle(clone_waker, self)
            }
        };

        trace!("Inserting clone {clone_id} back into the fork with state: {new_state:?}.");
        self.clones.insert(clone_id, new_state);
        poll_result
    }

    pub(crate) fn waker(&self, extra_waker: &Waker) -> Waker {
        let wakers = self
            .clones
            .iter()
            .filter(|(_clone_id, state)| state.should_still_see_base_item())
            .filter_map(|(_clone_id, state)| state.waker().clone())
            .chain(std::iter::once(extra_waker.clone()))
            .collect::<Vec<_>>();

        trace!("Found {} wakers.", wakers.len());

        Waker::from(Arc::new(SleepWaker { wakers }))
    }

    pub(crate) fn register(&mut self) -> usize {
        let min_available = self.next_clone_index;

        trace!("Registering clone {min_available}.");
        self.clones.insert(min_available, CloneState::default());

        self.next_clone_index += 1;
        min_available
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        self.clones.remove(&clone_id).unwrap();

        self.queue.retain(|item_index, _| {
            self.clones
                .values()
                .any(|state| state.should_still_see_item(*item_index))
        });
    }

    pub(crate) fn remaining_queued_items(&self, clone_id: usize) -> usize {
        self.queue
            .iter()
            .filter(|(item_index, _)| {
                self.clones
                    .get(&clone_id)
                    .unwrap()
                    .should_still_see_item(**item_index)
            })
            .count()
    }
}

impl<BaseStream> Deref for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = BaseStream;

    fn deref(&self) -> &Self::Target {
        &self.base_stream
    }
}

pub(crate) struct SleepWaker {
    wakers: Vec<Waker>,
}

impl Wake for SleepWaker {
    fn wake(self: Arc<Self>) {
        trace!("Waking up all sleeping clones.");
        self.wakers.iter().for_each(Waker::wake_by_ref);
    }
}
