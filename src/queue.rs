use futures::Stream;
use log::trace;

use crate::{Fork, fork::CloneState};

#[derive(Debug, Clone)]
pub(crate) enum TotalQueue<Item> {
    ItemCloned { index: usize, item: Item },
    ItemPopped { index: usize, item: Item },
    Empty,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn has_lagging_siblings(&self, clone_index: usize) -> bool {
        self.queue.first_key_value().is_none()
            || self.queue.first_key_value().is_some_and(|(item_index, _)| {
                self.clones
                    .iter()
                    .filter(|(i, _)| **i != clone_index)
                    .filter(|(_, state)| {
                        state.last_seen.is_none()
                            || state.last_seen.is_some_and(|last| *item_index > last)
                    })
                    .count()
                    == 0
            })
    }

    pub(crate) fn pop_queue(&mut self, clone_id: usize) -> TotalQueue<Option<BaseStream::Item>> {
        if self.queue.is_empty() {
            TotalQueue::Empty
        } else if self.has_lagging_siblings(clone_id) {
            let first_entry = self.queue.pop_first().unwrap();
            TotalQueue::ItemPopped {
                index: first_entry.0,
                item: first_entry.1,
            }
        } else {
            let first = self.queue.first_key_value().unwrap();
            TotalQueue::ItemCloned {
                index: *first.0,
                item: first.1.clone(),
            }
        }
    }

    pub(crate) fn enqueue_new_item(
        &mut self,
        current_clone_id: usize,
        new_item: Option<&BaseStream::Item>,
    ) -> Option<usize> {
        if self.sleeping_siblings(current_clone_id) {
            trace!("Enqueuing item received while polling clone {current_clone_id}.");
            self.queue.insert(self.next_queue_index, new_item.cloned());
            let new_index = self.next_queue_index;
            self.clones
                .iter_mut()
                .filter(|(id, _)| **id != current_clone_id)
                .for_each(|(other_clone_id, other_clone)| {
                    if let CloneState::Sleeping = other_clone.state {
                        trace!("Waking up clone {other_clone_id}.");
                        other_clone.waker.take().unwrap().wake_by_ref();
                        other_clone.state = CloneState::Woken;

                        other_clone.last_seen = Some(self.next_queue_index);
                    } else {
                        trace!("The clone {other_clone_id} is not sleeping or unpolled.");
                    }
                });

            self.next_queue_index += 1;
            Some(new_index)
        } else {
            trace!("Clone {current_clone_id} is the only one active.");
            None
        }
    }

    fn sleeping_siblings(&mut self, current_clone_id: usize) -> bool {
        self.clones
            .iter()
            .filter(|(id, sibling)| {
                **id != current_clone_id && matches!(sibling.state, CloneState::Sleeping)
            })
            .count()
            > 0
    }

    /// Checks if the given index points to an item that is the youngest (the
    /// one with the largest index) in the queue.
    pub(crate) fn latest_item_on_queue(&self, item_index: usize) -> bool {
        self.queue.is_empty()
            || self
                .queue
                .keys()
                .all(|queue_index| *queue_index <= item_index)
    }

    pub(crate) fn n_queued_items(&self, clone_id: usize) -> usize {
        let state = self.clones.get(&clone_id).unwrap();

        match state.last_seen {
            Some(last_seen) => self.queue.range((last_seen + 1)..).count(),
            None => self.queue.len(),
        }
    }
}
