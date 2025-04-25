use core::ops::Deref;
use std::{
    collections::BTreeMap,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

pub struct SiblingClone {
    pub(crate) id: usize,
    last_seen: Option<usize>,
    waker: Option<Waker>,
    pub(crate) state: CloneState,
}

#[derive(Default)]
pub(crate) enum CloneState {
    #[default]
    Unpolled,
    Woken,
    Sleeping,
}

#[derive(Debug, Clone)]
enum TotalQueue<Item> {
    ItemCloned { index: usize, item: Item },
    ItemPopped { index: usize, item: Item },
    Empty,
}

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    base_stream: Pin<Box<BaseStream>>,
    queue: BTreeMap<usize, Option<BaseStream::Item>>,
    clones: BTreeMap<usize, SiblingClone>,
    next_queue_index: usize,
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
        }
    }

    fn has_lagging_siblings(&self, clone_index: usize) -> bool {
        let item_index = *self.queue.first_key_value().unwrap().0;
        self.clones
            .iter()
            .filter(|(i, _)| **i != clone_index)
            .filter(|(_, state)| {
                state.last_seen.is_none() || state.last_seen.is_some_and(|last| item_index > last)
            })
            .count()
            == 0
    }

    fn pop_queue(&mut self, clone_id: usize) -> TotalQueue<Option<BaseStream::Item>> {
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

    fn enqueue_wake_siblings(
        &mut self,
        current_clone_id: usize,
        item: Option<&BaseStream::Item>,
    ) -> Option<usize> {
        if self.sleeping_siblings(current_clone_id) {
            trace!("Enqueuing item received while polling clone {current_clone_id}.");
            self.queue.insert(self.next_queue_index, item.cloned());
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

    fn newest_on_queue(&self, item_index: usize) -> bool {
        self.queue.is_empty()
            || self
                .queue
                .keys()
                .all(|queue_index| *queue_index >= item_index)
    }

    fn handle_woken_state(
        &mut self,
        state: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self.pop_queue(state.id).clone() {
            TotalQueue::ItemPopped {
                item,
                index: peeked_index,
            } => {
                state.last_seen = Some(peeked_index);
                if !self.newest_on_queue(peeked_index) {
                    state.state = CloneState::Sleeping;
                    state.waker = Some(clone_waker.clone());
                }
                Poll::Ready(item)
            }
            TotalQueue::ItemCloned {
                index: popped_index,
                item,
            } => {
                state.last_seen = self.enqueue_wake_siblings(state.id, item.as_ref());
                if !self.newest_on_queue(popped_index) {
                    state.waker = Some(clone_waker.clone());
                    state.last_seen = Some(popped_index);
                }
                Poll::Ready(item.clone())
            }
            TotalQueue::Empty => self.poll_base_stream(state, clone_waker),
        }
    }

    fn poll_base_stream(
        &mut self,
        state: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(clone_waker))
        {
            Poll::Pending => {
                state.state = CloneState::Sleeping;
                state.waker = Some(clone_waker.clone());
                Poll::Pending
            }
            Poll::Ready(item) => {
                state.state = CloneState::Unpolled;
                self.enqueue_wake_siblings(state.id, item.as_ref());
                Poll::Ready(item)
            }
        }
    }

    fn handle_sleeping_state(
        &mut self,
        state: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        if let CloneState::Sleeping = state.state {
            if !state.waker.as_ref().unwrap().will_wake(clone_waker) {
                state.waker = Some(clone_waker.clone());
            }
            if state.last_seen.is_none()
                || state
                    .last_seen
                    .is_some_and(|last| !self.newest_on_queue(last))
            {
                self.poll_base_stream(state, clone_waker)
            } else {
                match self.pop_queue(state.id).clone() {
                    TotalQueue::ItemPopped {
                        item,
                        index: peeked_index,
                    } => {
                        state.last_seen = Some(peeked_index);
                        Poll::Ready(item)
                    }
                    TotalQueue::ItemCloned {
                        index: popped_index,
                        item,
                    } => {
                        self.enqueue_wake_siblings(state.id, item.as_ref());
                        state.last_seen = Some(popped_index);
                        Poll::Ready(item.clone())
                    }
                    TotalQueue::Empty => Poll::Pending,
                }
            }
        } else {
            unreachable!()
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the split.");
        let mut state = self.clones.remove(&clone_id).unwrap();

        let poll = match &mut state.state {
            CloneState::Woken => self.handle_woken_state(&mut state, clone_waker),
            CloneState::Unpolled => self.poll_base_stream(&mut state, clone_waker),
            CloneState::Sleeping => self.handle_sleeping_state(&mut state, clone_waker),
        };

        self.clones.insert(clone_id, state);
        poll
    }

    pub(crate) fn register(&mut self) -> usize {
        let min_available = (0..)
            .filter(|n| !self.clones.contains_key(n))
            .nth(0)
            .unwrap();

        trace!("Registering clone {min_available}.");
        self.clones.insert(
            min_available,
            SiblingClone {
                state: CloneState::default(),
                id: min_available,
                last_seen: None,
                waker: None,
            },
        );
        min_available
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        self.clones.remove(&clone_id).unwrap();

        self.queue.retain(|item_index, _| {
            self.clones.values().any(|state| {
                state
                    .last_seen
                    .is_some_and(|last_index| *item_index > last_index)
            })
        });
    }

    pub(crate) fn n_queued_items(&self, clone_id: usize) -> usize {
        let state = self.clones.get(&clone_id).unwrap();

        match state.last_seen {
            Some(last_seen) => self.queue.range((last_seen + 1)..).count(),
            None => self.queue.len(),
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
