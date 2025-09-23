use core::ops::Deref;
use std::{
    iter,
    pin::Pin,
    sync::Arc,
    task::{Poll, Wake, Waker},
};

use futures::Stream;
use log::{debug, trace, warn};

use crate::{
    error::{CloneStreamError, Result},
    ring_queue::RingQueue,
    states::CloneState,
};

/// Maximum number of clones that can be registered simultaneously.
const MAX_CLONE_COUNT: usize = 65536;

/// Maximum number of items that can be queued simultaneously.
const MAX_QUEUE_SIZE: usize = 1024 * 1024;

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
    pub(crate) clones: Vec<Option<CloneState>>,
    available_clone_indices: Vec<usize>,
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
            clones: Vec::new(),
            queue: RingQueue::new(config.max_queue_size),
            available_clone_indices: Vec::new(),
            config,
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        let mut current_state = self.clones[clone_id].take().unwrap();
        debug!("State of clone {clone_id} is {current_state:?}.");

        let poll_result = current_state.step(clone_id, clone_waker, self);

        debug!("Clone {clone_id} transitioned to {current_state:?}.");
        self.clones[clone_id] = Some(current_state);
        poll_result
    }

    pub(crate) fn waker(&self, extra_waker: &Waker) -> Waker {
        let clone_wakers: Vec<Waker> = self
            .clones
            .iter()
            .flatten()
            .filter(|state| state.should_still_see_base_item())
            .filter_map(super::states::CloneState::waker)
            .collect();

        let waker_count = clone_wakers.len() + 1;

        // Avoid Arc allocation for single waker
        if waker_count == 1 {
            extra_waker.clone()
        } else {
            let all_wakers = clone_wakers
                .into_iter()
                .chain(iter::once(extra_waker.clone()))
                .collect();
            Waker::from(Arc::new(MultiWaker { wakers: all_wakers }))
        }
    }

    /// Count the number of active clones
    pub(crate) fn active_clone_count(&self) -> usize {
        self.clones.iter().filter(|s| s.is_some()).count()
    }

    /// Check if a clone exists and is active
    fn clone_exists(&self, clone_id: usize) -> bool {
        clone_id < self.clones.len() && self.clones[clone_id].is_some()
    }

    /// Register a new clone and return its ID
    pub(crate) fn register(&mut self) -> Result<usize> {
        // Try to reuse an available index first
        if let Some(reused_id) = self.available_clone_indices.pop() {
            trace!("Registering clone {reused_id} (reused index).");
            self.clones[reused_id] = Some(CloneState::default());
            return Ok(reused_id);
        }

        if self.active_clone_count() >= self.config.max_clone_count {
            return Err(CloneStreamError::MaxClonesExceeded {
                current_count: self.active_clone_count(),
                max_allowed: self.config.max_clone_count,
            });
        }

        let clone_id = self.clones.len();
        trace!("Registering clone {clone_id} (new index).");
        self.clones.push(Some(CloneState::default()));
        Ok(clone_id)
    }

    pub(crate) fn remaining_queued_items(&self, clone_id: usize) -> usize {
        (&self.queue)
            .into_iter()
            .map(|(item_index, _)| item_index)
            .filter(|&item_index| self.clone_should_still_see_item(clone_id, item_index))
            .count()
    }

    pub(crate) fn has_other_clones_waiting(&self, exclude_clone_id: usize) -> bool {
        self.clones.iter().enumerate().any(|(clone_id, state_opt)| {
            clone_id != exclude_clone_id
                && state_opt
                    .as_ref()
                    .is_some_and(super::states::CloneState::should_still_see_base_item)
        })
    }

    /// Find queue items that no active clone needs anymore
    fn find_unneeded_queue_items(&self) -> impl Iterator<Item = usize> {
        (&self.queue).into_iter().filter_map(|(item_index, _)| {
            let is_needed = (0..self.clones.len())
                .any(|clone_id| self.clone_should_still_see_item(clone_id, item_index));
            (!is_needed).then_some(item_index)
        })
    }

    pub(crate) fn clone_should_still_see_item(
        &self,
        clone_id: usize,
        queue_item_index: usize,
    ) -> bool {
        if let Some(Some(state)) = self.clones.get(clone_id) {
            match state {
                CloneState::QueueEmptyPending { .. } => true,
                CloneState::AllSeenPending {
                    last_seen_index, ..
                } => self.queue.is_newer_than(queue_item_index, *last_seen_index),
                CloneState::UnseenReady { unseen_index } => {
                    !self.queue.is_newer_than(queue_item_index, *unseen_index)
                }
                CloneState::QueueEmpty | CloneState::AllSeen => false,
            }
        } else {
            false
        }
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");

        if !self.clone_exists(clone_id) {
            log::warn!("Attempted to unregister clone {clone_id} that was not registered");
            return;
        }

        self.clones[clone_id] = None;
        self.available_clone_indices.push(clone_id);

        self.cleanup_unneeded_queue_items();
        trace!("Unregister of clone {clone_id} complete.");
    }

    fn cleanup_unneeded_queue_items(&mut self) {
        if self.active_clone_count() == 0 {
            self.queue.clear();
            return;
        }

        self.find_unneeded_queue_items()
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|item_index| {
                self.queue.remove(item_index);
            });
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
