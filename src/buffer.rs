use std::{collections::VecDeque, task::Waker};

use log::trace;

pub struct TaskBuffer<Item> {
    pub task_waker: Waker,
    pub item_queue: VecDeque<Item>,
}

impl<Item> TaskBuffer<Item> {
    pub fn new(task_waker: Waker) -> Self {
        Self {
            task_waker,
            item_queue: VecDeque::new(),
        }
    }
}

/// An object representing the tasks of the forks that polled but were suspended.
/// The tasks are represented by their initial waker.
/// The items are buffered in a queue for each task.
#[derive(Default)]
pub struct SuspendedForks<Item> {
    /// Buffers for each task that is waiting for an item.
    task_buffers: VecDeque<TaskBuffer<Item>>,
    /// Maximum number of items that can be buffered for each task.
    /// If `None`, there is no limit.
    max_buffered: Option<usize>,
}

impl<Item> SuspendedForks<Item>
where
    Item: Clone,
{
    pub fn new(max_buffered: Option<usize>) -> Self {
        assert!(
            max_buffered.is_none_or(|s| s != 0),
            "Buffer size should be larger than one."
        );
        Self {
            task_buffers: VecDeque::new(),
            max_buffered,
        }
    }

    pub fn clear(&mut self) {
        self.task_buffers.clear();
    }
    pub fn append(&mut self, item: Item, waker: &Waker) {
        self.task_buffers
            .iter_mut()
            .filter(|fork| !fork.task_waker.will_wake(waker))
            .for_each(|fork| {
                if let Some(max) = self.max_buffered {
                    while fork.item_queue.len() >= max {
                        trace!("Buffer is full, removing the oldest item");
                        fork.item_queue.pop_front();
                    }

                    trace!("Adding item to the buffer");
                    fork.item_queue.push_back(item.clone());
                } else {
                    trace!("Adding item to the buffer");
                    fork.item_queue.push_back(item.clone());
                }
            });
    }

    pub fn n_cached(&self, waker: &Waker) -> usize {
        self.task_buffers
            .iter()
            .find(|fork| fork.task_waker.will_wake(waker))
            .map_or(0, |fork| fork.item_queue.len())
    }

    pub fn earliest_item(&mut self, waker: &Waker) -> Option<Item> {
        self.task_buffers
            .iter_mut()
            .find(|fork| fork.task_waker.will_wake(waker))?
            .item_queue
            .pop_front()
    }

    pub fn wake_all(&self) {
        for fork in &self.task_buffers {
            fork.task_waker.wake_by_ref();
        }
    }

    pub fn insert_buffer(&mut self, waker: Waker) {
        if !self
            .task_buffers
            .iter()
            .any(|fork| fork.task_waker.will_wake(&waker))
        {
            self.task_buffers.push_back(TaskBuffer::new(waker));
        }
    }

    pub fn remove_buffer_if_empty(&mut self, waker: &Waker) {
        if let Some(index) = self
            .task_buffers
            .iter()
            .position(|fork| fork.task_waker.will_wake(waker))
        {
            if self.task_buffers[index].item_queue.is_empty() {
                trace!("Removing empty buffer");
                self.task_buffers.remove(index);
            }
        }
    }
}
