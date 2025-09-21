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
    next_clone_id: usize,
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
            available_clone_indices: BTreeSet::new(),
            next_clone_id: 0,
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
        } = current_state.handle(clone_id, clone_waker, self);

        debug!("Clone {clone_id} transitioned from {current_state:?} to {new_state:?}.");
        self.clones.insert(clone_id, new_state);
        poll_result
    }

    pub(crate) fn waker(&self, extra_waker: &Waker) -> Waker {
        let capacity = self.clones.len() + 1;
        let mut wakers = Vec::with_capacity(capacity);

        for state in self.clones.values() {
            if state.should_still_see_base_item()
                && let Some(waker) = state.waker()
            {
                wakers.push(waker);
            }
        }
        wakers.push(extra_waker.clone());

        Waker::from(Arc::new(MultiWaker { wakers }))
    }

    /// Register a new clone and return its ID
    pub(crate) fn register(&mut self) -> Result<usize> {
        if let Some(reused_id) = self.available_clone_indices.pop_first() {
            trace!("Registering clone {reused_id} (reused index).");
            self.clones.insert(reused_id, CloneState::default());
            return Ok(reused_id);
        }

        if self.clones.len() >= self.config.max_clone_count {
            return Err(CloneStreamError::MaxClonesExceeded {
                current_count: self.clones.len(),
                max_allowed: self.config.max_clone_count,
            });
        }

        let clone_id = self.next_clone_id;
        self.next_clone_id = (self.next_clone_id + 1) % self.config.max_clone_count;

        trace!("Registering clone {clone_id} (new index).");
        self.clones.insert(clone_id, CloneState::default());
        Ok(clone_id)
    }

    pub(crate) fn remaining_queued_items(&self, clone_id: usize) -> usize {
        (&self.queue)
            .into_iter()
            .filter(|(item_index, _)| self.clone_should_still_see_item(clone_id, *item_index))
            .count()
    }

    pub(crate) fn has_other_clones_waiting(&self, exclude_clone_id: usize) -> bool {
        self.clones.iter().any(|(clone_id, state)| {
            *clone_id != exclude_clone_id && state.should_still_see_base_item()
        })
    }

    pub(crate) fn clone_should_still_see_item(
        &self,
        clone_id: usize,
        queue_item_index: usize,
    ) -> bool {
        let state = self.clones.get(&clone_id).unwrap();
        match state {
            CloneState::QueueEmptyThenBasePending(_) => true,
            CloneState::NoUnseenQueuedThenBasePending(no_unseen_queued_then_base_pending) => {
                self.queue.is_strictly_newer_then(
                    queue_item_index,
                    no_unseen_queued_then_base_pending.most_recent_queue_item_index,
                )
            }
            CloneState::UnseenQueuedItemReady(unseen_queued_item_ready) => {
                !self.queue.is_strictly_newer_then(
                    queue_item_index,
                    unseen_queued_item_ready.unseen_ready_queue_item_index,
                )
            }
            CloneState::NeverPolled(_)
            | CloneState::QueueEmptyThenBaseReady(_)
            | CloneState::NoUnseenQueuedThenBaseReady(_) => false,
        }
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        if self.clones.remove(&clone_id).is_none() {
            log::warn!("Attempted to unregister clone {clone_id} that was not registered");
            return;
        }

        if !self.available_clone_indices.insert(clone_id) {
            log::warn!("Clone index {clone_id} was already in available pool");
        }

        self.cleanup_unneeded_queue_items();
        trace!("Unregister of clone {clone_id} complete.");
    }

    fn cleanup_unneeded_queue_items(&mut self) {
        // If no clones remaining, clear the entire queue
        if self.clones.is_empty() {
            while self.queue.oldest_with_index().is_some() {}
            return;
        }

        let mut items_to_remove = Vec::new();

        for (item_index, _) in &self.queue {
            let mut is_needed = false;

            for &clone_id in self.clones.keys() {
                if self.clone_should_still_see_item(clone_id, item_index) {
                    is_needed = true;
                    break; // Early exit - at least one clone needs it
                }
            }

            if !is_needed {
                items_to_remove.push(item_index);
            }
        }

        for item_index in items_to_remove {
            self.queue.remove(item_index);
        }
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
