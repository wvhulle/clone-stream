use futures::Stream;
use log::trace;

use crate::{
    Fork,
    transitions::{CloneState, ReadyToPop, Suspended},
};

#[derive(Debug, Clone)]
pub(crate) enum QueuePopState<Item> {
    ItemCloned { index: usize, item: Item },
    ItemPopped { index: usize, item: Item },
    Empty,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn has_lagging_siblings(&self) -> bool {
        self.queue.first_key_value().is_none()
            || self.queue.first_key_value().is_some_and(|(item_index, _)| {
                self.clones
                    .iter()
                    .filter(|(_, clone)| clone.older_than(*item_index))
                    .count()
                    == 0
            })
    }

    pub(crate) fn pop_queue(&mut self) -> QueuePopState<Option<BaseStream::Item>> {
        if self.queue.is_empty() {
            QueuePopState::Empty
        } else if !self.has_lagging_siblings() {
            let first_entry = self.queue.pop_first().unwrap();
            QueuePopState::ItemPopped {
                index: first_entry.0,
                item: first_entry.1,
            }
        } else {
            let first = self.queue.first_key_value().unwrap();
            QueuePopState::ItemCloned {
                index: *first.0,
                item: first.1.clone(),
            }
        }
    }

    fn clone_has_sleeping_siblings(&mut self) -> bool {
        self.clones
            .iter()
            .filter(|(_, sibling)| matches!(sibling, CloneState::Suspended { .. }))
            .count()
            > 0
    }

    /// Enqueues any new item that is received while polling the base stream for
    /// a particular clone. This will push the item on the shared queue and
    /// wake up any sleeping clones so that they can read/clone the new item on
    /// the queue (after their next wakeup).
    pub(crate) fn enqueue_new_item(
        &mut self,
        new_item: Option<&BaseStream::Item>,
    ) -> Option<usize> {
        if self.clone_has_sleeping_siblings() {
            self.queue.insert(self.next_queue_index, new_item.cloned());
            let new_index = self.next_queue_index;
            self.clones
                .iter_mut()
                .for_each(|(other_clone_id, other_clone)| {
                    if let CloneState::Suspended(Suspended { waker, .. }) = other_clone {
                        trace!("Waking up clone {other_clone_id}.");
                        waker.wake_by_ref();

                        *other_clone = CloneState::ReadyToPop(ReadyToPop {
                            last_seen: Some(new_index),
                        });
                    } else {
                        trace!("The clone {other_clone_id} is not sleeping or unpolled.");
                    }
                });

            self.next_queue_index += 1;
            Some(new_index)
        } else {
            trace!("No clones are sleeping.");
            None
        }
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

        match state {
            CloneState::Suspended(Suspended { last_seen, .. })
            | CloneState::ReadyToPop(ReadyToPop { last_seen, .. }) => match last_seen {
                Some(last_seen) => self.queue.range((last_seen + 1)..).count(),
                None => self.queue.len(),
            },
            CloneState::UpToDate => 0,
        }
    }
}
