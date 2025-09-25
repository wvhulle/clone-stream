use core::ops::Deref;
use std::{
    iter,
    pin::Pin,
    sync::Arc,
    task::{Poll, Wake, Waker},
};

use futures::Stream;
use log::{debug, trace, warn};

use crate::{error::Result, registry::CloneRegistry, ring_queue::RingQueue};

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
    pub(crate) clone_registry: CloneRegistry,
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
            clone_registry: CloneRegistry::new(config.max_clone_count),
            queue: RingQueue::new(config.max_queue_size),
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        let mut current_state = self.clone_registry.take(clone_id).unwrap();
        debug!("State of clone {clone_id} is {current_state:?}.");

        let poll_result = current_state.step(clone_id, clone_waker, self);

        debug!("Clone {clone_id} transitioned to {current_state:?}.");
        self.clone_registry.restore(clone_id, current_state);
        poll_result
    }

    pub(crate) fn waker(&self, extra_waker: &Waker) -> Waker {
        let clone_wakers = self.clone_registry.collect_wakers_needing_base_item();
        trace!(
            "There are {} clone wakers needing base item. Adding one more",
            clone_wakers.len()
        );
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
        self.clone_registry.active_count()
    }

    /// Register a new clone and return its ID
    pub(crate) fn register(&mut self) -> Result<usize> {
        self.clone_registry.register()
    }

    pub(crate) fn remaining_queued_items(&self, clone_id: usize) -> usize {
        (&self.queue)
            .into_iter()
            .map(|(item_index, _)| item_index)
            .filter(|&item_index| self.should_clone_see_item(clone_id, item_index))
            .count()
    }

    pub(crate) fn has_other_clones_waiting(&self, exclude_clone_id: usize) -> bool {
        self.clone_registry
            .has_other_clones_waiting(exclude_clone_id)
    }

    pub(crate) fn should_clone_see_item(&self, clone_id: usize, queue_item_index: usize) -> bool {
        if let Some(state) = self.clone_registry.get_clone_state(clone_id) {
            match state {
                crate::states::CloneState::Initial
                | crate::states::CloneState::QueueEmptyPending { .. } => true,
                crate::states::CloneState::AllSeenPending {
                    last_seen_index, ..
                } => self.queue.is_newer_than(queue_item_index, *last_seen_index),
                crate::states::CloneState::PreviouslySawOnQueue {
                    last_seen_queue_index: unseen_index,
                } => !self.queue.is_newer_than(queue_item_index, *unseen_index),
                crate::states::CloneState::QueueEmpty | crate::states::CloneState::AllSeen => false,
            }
        } else {
            false
        }
    }

    /// Find queue items that no active clone needs anymore
    fn find_unneeded_queue_items(&self) -> impl Iterator<Item = usize> {
        (&self.queue).into_iter().filter_map(|(item_index, _)| {
            let is_needed = (0..self.clone_registry.len())
                .any(|clone_id| self.should_clone_see_item(clone_id, item_index));
            (!is_needed).then_some(item_index)
        })
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        self.clone_registry.unregister(clone_id);
        self.cleanup_unneeded_queue_items();
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
