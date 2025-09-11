use core::ops::Deref;
use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::Arc,
    task::{Poll, Wake, Waker},
};

use futures::Stream;
use log::trace;

use crate::states::{CloneState, NewStateAndPollResult, StateHandler};

/// Maximum number of clones that can be registered simultaneously.
/// This prevents overflow of clone indices and limits memory usage.
/// 65536 clones should be more than sufficient for any practical use case.
const MAX_CLONE_COUNT: usize = 65536;

/// Maximum number of items that can be queued simultaneously.
/// This prevents overflow of queue indices and limits memory usage.
/// 1MB queue indices should handle most streaming scenarios comfortably.
const MAX_QUEUE_SIZE: usize = 1024 * 1024;

/// Buffer size before attempting queue index reset when approaching overflow.
const QUEUE_RESET_THRESHOLD: usize = 1000;

/// Configuration for Fork behavior.
#[derive(Debug, Clone, Copy)]
pub struct ForkConfig {
    /// Maximum number of clones allowed.
    pub max_clone_count: usize,
    /// Maximum queue size before panic.
    pub max_queue_size: usize,
}

impl Default for ForkConfig {
    fn default() -> Self {
        Self {
            max_clone_count: MAX_CLONE_COUNT,
            max_queue_size: MAX_QUEUE_SIZE,
        }
    }
}

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) base_stream: Pin<Box<BaseStream>>,
    pub(crate) queue: BTreeMap<usize, Option<BaseStream::Item>>,
    pub(crate) clones: BTreeMap<usize, CloneState>,
    next_clone_index: usize,
    pub(crate) next_queue_index: usize,
    config: ForkConfig,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self::with_config(base_stream, ForkConfig::default())
    }

    pub(crate) fn with_config(base_stream: BaseStream, config: ForkConfig) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            clones: BTreeMap::default(),
            queue: BTreeMap::new(),
            next_queue_index: 0,
            next_clone_index: 0,
            config,
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
        } = current_state.handle(clone_waker, self);

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
        // Check for overflow before incrementing
        assert!(
            self.next_clone_index < self.config.max_clone_count,
            "Maximum number of clones ({}) exceeded", self.config.max_clone_count
        );

        let clone_id = self.next_clone_index;

        trace!("Registering clone {clone_id}.");
        self.clones.insert(clone_id, CloneState::default());

        self.next_clone_index += 1;
        clone_id
    }

    /// Allocates a new queue index with overflow protection.
    /// This method should be called instead of directly incrementing `next_queue_index`.
    pub(crate) fn allocate_queue_index(&mut self) -> usize {
        // If we're approaching overflow and the queue is empty, reset to 0
        // This helps with long-running applications that create many temporary items
        if self.next_queue_index >= self.config.max_queue_size - QUEUE_RESET_THRESHOLD && self.queue.is_empty() {
            self.next_queue_index = 0;
        }

        // Check for overflow before incrementing
        assert!(
            self.next_queue_index < self.config.max_queue_size,
            "Queue index overflow: maximum queue size ({}) exceeded", self.config.max_queue_size
        );

        let allocated_index = self.next_queue_index;
        self.next_queue_index += 1;
        allocated_index
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
