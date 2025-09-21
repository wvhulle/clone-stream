use core::ops::Deref;
use std::{
    collections::{BTreeMap, BTreeSet},
    pin::Pin,
    sync::Arc,
    task::{Poll, Wake, Waker},
};

use futures::Stream;
use log::{debug, trace, warn};

use crate::{
    error::{CloneStreamError, Result},
    ring_queue::RingQueue,
    states::{CloneState, NewStateAndPollResult, StateHandler},
};

/// Maximum number of clones that can be registered simultaneously.
const MAX_CLONE_COUNT: usize = 65536;

/// Maximum number of items that can be queued simultaneously.
const MAX_QUEUE_SIZE: usize = 1024 * 1024;

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
    pub(crate) queue: RingQueue<Option<BaseStream::Item>>,
    pub(crate) clones: BTreeMap<usize, CloneState>,
    available_clone_indices: BTreeSet<usize>,
    latest_cached_item_index: Option<usize>,
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
            queue: RingQueue::new(config.max_queue_size),
            latest_cached_item_index: None,
            available_clone_indices: BTreeSet::new(),
            config,
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        let current_state = self.clones.remove(&clone_id).unwrap();
        debug!("State of clone {clone_id} is {current_state:?}.");
        let NewStateAndPollResult {
            poll_result,
            new_state,
        } = current_state.handle(clone_waker, self);

        debug!("Clone {clone_id} transitioned from {current_state:?} to {new_state:?}.");
        self.clones.insert(clone_id, new_state);
        poll_result
    }

    pub(crate) fn waker(&self, extra_waker: &Waker) -> Waker {
        let existing_wakers = self
            .clones
            .iter()
            .filter(|(_clone_id, state)| state.should_still_see_base_item())
            .filter_map(|(_clone_id, state)| state.waker().clone());

        let new_total_wakers = existing_wakers
            .chain(std::iter::once(extra_waker.clone()))
            .collect::<Vec<_>>();

        Waker::from(Arc::new(MultiWaker {
            wakers: new_total_wakers,
        }))
    }

    /// Register a new clone and return its ID
    pub(crate) fn register(&mut self) -> Result<usize> {
        if let Some(reused_id) = self.available_clone_indices.pop_first() {
            trace!("Registering clone {reused_id} (reused index).");
            self.clones.insert(reused_id, CloneState::default());
            return Ok(reused_id);
        }

        // Derive the next new index by finding the lowest unused index
        let next_clone_index = (0..self.config.max_clone_count)
            .find(|&id| !self.clones.contains_key(&id))
            .ok_or(CloneStreamError::MaxClonesExceeded {
                current_count: self.clones.len(),
                max_allowed: self.config.max_clone_count,
            })?;

        trace!("Registering clone {next_clone_index} (new index).");
        self.clones.insert(next_clone_index, CloneState::default());
        Ok(next_clone_index)
    }

    pub(crate) fn remaining_queued_items(&self, clone_id: usize) -> usize {
        (&self.queue)
            .into_iter()
            .filter(|(item_index, _)| self.clone_should_still_see_item(clone_id, *item_index))
            .count()
    }

    /// Checks if a clone should still see an item, using proper ring buffer ordering.
    pub(crate) fn clone_should_still_see_item(
        &self,
        clone_id: usize,
        queue_item_index: usize,
    ) -> bool {
        let state = self.clones.get(&clone_id).unwrap();
        match state {
            CloneState::QueueEmptyThenBasePending(_) => true,
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                // Use ring buffer ordering instead of direct comparison
                self.queue.is_after(
                    queue_item_index,
                    no_unseen_queued_then_base_pending.most_recent_queue_item_index,
                )
            }
            CloneState::NeverPolled(_)
            | CloneState::QueueEmptyThenBaseReady(_)
            | CloneState::NoUnseenQueuedThenBaseReady(_)
            | CloneState::UnseenQueuedItemReady(_) => false,
        }
    }

    /// Finds the next queue index after the given index, using ring buffer ordering.
    pub(crate) fn find_next_queue_index_after(&self, after_index: usize) -> Option<usize> {
        self.queue
            .keys()
            .find(|queue_index| self.queue.is_after(**queue_index, after_index))
            .copied()
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        if self.clones.remove(&clone_id).is_none() {
            log::warn!("Attempted to unregister clone {clone_id} that was not registered");
            return;
        }

        // Insert the index back to the available pool - BTreeSet handles ordering
        // automatically
        if !self.available_clone_indices.insert(clone_id) {
            log::warn!("Clone index {clone_id} was already in available pool");
        }
        trace!("Removing unneeded items from the queue.");
        // Collect items to remove to avoid borrowing issues
        let items_to_remove: Vec<usize> = {
            let mut to_remove = Vec::new();
            for (item_index, _) in &self.queue {
                trace!("Checking if item {item_index} is still needed on the queue.");
                let is_needed = self
                    .clones
                    .iter()
                    .any(|(clone_id, _)| self.clone_should_still_see_item(*clone_id, item_index));
                if !is_needed {
                    to_remove.push(item_index);
                }
            }
            to_remove
        };

        // Remove the unneeded items
        for item_index in items_to_remove {
            self.queue.remove(item_index);
        }
        trace!("Unregister of clone {clone_id} complete.");
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

pub(crate) struct MultiWaker {
    wakers: Vec<Waker>,
}

impl Wake for MultiWaker {
    fn wake(self: Arc<Self>) {
        warn!("New data arrived in source stream, waking up sleeping clones.");
        self.wakers.iter().for_each(Waker::wake_by_ref);
    }
}
