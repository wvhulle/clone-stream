use std::{collections::VecDeque, task::Waker};

pub struct TaskItemQueue<Item> {
    pub task_waker: Waker,
    pub item_queue: VecDeque<Item>,
}

impl<Item> TaskItemQueue<Item> {
    pub fn new(task_waker: Waker) -> Self {
        Self {
            task_waker,
            item_queue: VecDeque::new(),
        }
    }

    pub fn shrink(&mut self, max: usize) {
        while self.item_queue.len() >= max {
            self.item_queue.pop_front();
        }
    }
}

/// An object representing the tasks of the forks that polled for a next item
/// but were suspended.
/// The tasks are represented by their initial waker.
/// The items are buffered in a queue for each task.
#[derive(Default)]
pub struct SuspendedForks<Item> {
    /// Buffers for each task (one per fork) that is waiting for the next `Option<Stream::Item>`.
    task_item_queues: VecDeque<TaskItemQueue<Item>>,
    /// Maximum number of items that can be buffered for each task.
    /// If `None`, there is no limit.
    max_buffered_items: Option<usize>,
}

impl<Item> SuspendedForks<Item>
where
    Item: Clone,
{
    pub fn new(max_buffered_items: Option<usize>) -> Self {
        assert!(
            max_buffered_items.is_none_or(|s| s != 0),
            "Buffer size should be none (unrestricted) or larger than zero."
        );
        Self {
            task_item_queues: VecDeque::new(),
            max_buffered_items,
        }
    }

    pub fn clear(&mut self) {
        self.task_item_queues.clear();
    }
    pub fn append(&mut self, item: Item, waker: &Waker) {
        self.task_item_queues
            .iter_mut()
            .filter(|fork| !fork.task_waker.will_wake(waker))
            .for_each(|fork| {
                if let Some(max) = self.max_buffered_items {
                    fork.shrink(max);
                }
                fork.item_queue.push_back(item.clone());
            });
    }

    pub fn items_remaining(&self, waker: &Waker) -> usize {
        self.task_item_queues
            .iter()
            .find(|fork| fork.task_waker.will_wake(waker))
            .map_or(0, |fork| fork.item_queue.len())
    }

    pub fn earliest_item(&mut self, waker: &Waker) -> Option<Item> {
        self.task_item_queues
            .iter_mut()
            .find(|fork| fork.task_waker.will_wake(waker))?
            .item_queue
            .pop_front()
    }

    pub fn wake_all(&self) {
        for fork in &self.task_item_queues {
            fork.task_waker.wake_by_ref();
        }
    }

    pub fn insert_empty_queue(&mut self, waker: Waker) {
        if !self
            .task_item_queues
            .iter()
            .any(|fork| fork.task_waker.will_wake(&waker))
        {
            self.task_item_queues.push_back(TaskItemQueue::new(waker));
        }
    }

    pub fn forget_if_queue_empty(&mut self, waker: &Waker) {
        if let Some(index) = self
            .task_item_queues
            .iter()
            .position(|fork| fork.task_waker.will_wake(waker))
        {
            if self.task_item_queues[index].item_queue.is_empty() {
                self.task_item_queues.remove(index);
            }
        }
    }
}
