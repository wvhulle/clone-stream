use std::{
    collections::BTreeMap,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::Fuse};
use log::trace;

#[derive(Default)]
pub enum CloneTaskState {
    #[default]
    Unpolled,
    Woken {
        last_seen: Option<usize>,
    },
    Sleeping {
        waker: Waker,
        last_seen_queued_item: Option<usize>,
    },
}

impl CloneTaskState {
    pub fn n_queued_items(&self) -> usize {
        match self {
            CloneTaskState::Unpolled => 0,
            CloneTaskState::Woken {
                last_seen: remaining,
                ..
            }
            | CloneTaskState::Sleeping {
                last_seen_queued_item: remaining,
                ..
            } => remaining.unwrap_or_default(),
        }
    }

    pub fn polled_once(&self) -> bool {
        match self {
            CloneTaskState::Unpolled => false,
            CloneTaskState::Woken { .. } | CloneTaskState::Sleeping { .. } => true,
        }
    }
}

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub base_stream: Pin<Box<Fuse<BaseStream>>>,
    pub queue: BTreeMap<usize, Option<BaseStream::Item>>,
    pub clones: BTreeMap<usize, CloneTaskState>,
    pub next_queue_index: usize,
}

#[derive(Debug, Clone)]
pub enum TotalQueue<Item> {
    ItemCloned { index: usize, item: Item },
    ItemPopped { index: usize, item: Item },
    Empty,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream.fuse()),
            clones: BTreeMap::default(),
            queue: BTreeMap::new(),
            next_queue_index: 0,
        }
    }

    fn not_yet_seen_by_all(&self, clone_index: usize) -> bool {
        let item_index = *self.queue.first_key_value().unwrap().0;
        self.clones
            .iter()
            .filter(|(i, _)| **i != clone_index)
            .filter(|(_, state)| match state {
                CloneTaskState::Woken {
                    last_seen: last_seen_queued_item,
                    ..
                } => {
                    last_seen_queued_item.is_none()
                        || last_seen_queued_item.is_some_and(|last| item_index > last)
                }
                CloneTaskState::Sleeping {
                    last_seen_queued_item,
                    ..
                } => {
                    last_seen_queued_item.is_none()
                        || last_seen_queued_item.is_some_and(|last| item_index > last)
                }
                CloneTaskState::Unpolled => false,
            })
            .count()
            == 0
    }

    fn pop(&mut self, clone_id: usize) -> TotalQueue<Option<BaseStream::Item>> {
        if self.queue.is_empty() {
            TotalQueue::Empty
        } else if self.not_yet_seen_by_all(clone_id) {
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

    fn enqueue(
        &mut self,
        current_clone_id: usize,
        item: Option<&BaseStream::Item>,
    ) -> Option<usize> {
        if self
            .clones
            .iter()
            .filter(|(id, c)| {
                **id != current_clone_id && matches!(c, CloneTaskState::Sleeping { .. })
            })
            .count()
            > 0
        {
            trace!("Enqueuing item received while polling clone {current_clone_id}.");
            self.queue.insert(self.next_queue_index, item.cloned());
            let new_index = self.next_queue_index;
            self.clones
                .iter_mut()
                .filter(|(id, _)| **id != current_clone_id)
                .for_each(|(other_clone_id, other_clone)| {
                    if let CloneTaskState::Sleeping { waker, .. } = other_clone {
                        trace!("Waking up clone {other_clone_id}.");
                        waker.wake_by_ref();
                        *other_clone = CloneTaskState::Woken {
                            last_seen: Some(self.next_queue_index),
                        };
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

    fn is_newest(&self, index: usize) -> bool {
        self.queue.is_empty() || self.queue.keys().min().copied().unwrap() >= index
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn update(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the split.");
        let mut state = self.clones.remove(&clone_id).unwrap();

        let poll = match &mut state {
            CloneTaskState::Woken { last_seen } => {
                trace!("Clone {clone_id} was already woken.");
                match self.pop(clone_id).clone() {
                    TotalQueue::ItemPopped {
                        item,
                        index: peeked_index,
                    } => {
                        trace!(
                            "An item was popped from the queue of clone {clone_id} and it was \
                             seen by all already."
                        );
                        *last_seen = Some(peeked_index);

                        if !self.is_newest(peeked_index) {
                            state = CloneTaskState::Sleeping {
                                waker: clone_waker.clone(),
                                last_seen_queued_item: Some(peeked_index),
                            };
                        }

                        Poll::Ready(item)
                    }
                    TotalQueue::ItemCloned {
                        index: popped_index,
                        item,
                    } => {
                        trace!(
                            "An item was popped from the queue of clone {clone_id} and it was not \
                             seen by all yet."
                        );

                        *last_seen = self.enqueue(clone_id, item.as_ref());

                        if !self.is_newest(popped_index) {
                            state = CloneTaskState::Sleeping {
                                waker: clone_waker.clone(),
                                last_seen_queued_item: Some(popped_index),
                            };
                        }

                        Poll::Ready(item.clone())
                    }

                    TotalQueue::Empty => {
                        trace!("The queue was empty for clone {clone_id}.");
                        match self
                            .base_stream
                            .poll_next_unpin(&mut Context::from_waker(clone_waker))
                        {
                            Poll::Pending => {
                                trace!("The base stream was not ready for clone {clone_id}.");
                                state = CloneTaskState::Sleeping {
                                    waker: clone_waker.clone(),
                                    last_seen_queued_item: None,
                                };

                                Poll::Pending
                            }
                            Poll::Ready(item) => {
                                trace!("The base stream was ready for clone {clone_id}.");
                                state = CloneTaskState::Unpolled;

                                self.enqueue(clone_id, item.as_ref());
                                Poll::Ready(item)
                            }
                        }
                    }
                }
            }
            CloneTaskState::Unpolled => {
                trace!("Fork {clone_id} was not polled yet.");
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(clone_waker))
                {
                    Poll::Pending => {
                        trace!("The base stream was not ready for clone {clone_id}.");
                        state = CloneTaskState::Sleeping {
                            waker: clone_waker.clone(),
                            last_seen_queued_item: None,
                        };
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        self.enqueue(clone_id, item.as_ref());
                        Poll::Ready(item)
                    }
                }
            }

            CloneTaskState::Sleeping {
                waker,
                last_seen_queued_item,
            } => {
                trace!("The clone {clone_id} was sleeping.");
                if !waker.will_wake(clone_waker) {
                    trace!("An old waker was registered for clone {clone_id}.");

                    waker.clone_from(clone_waker);
                }
                if last_seen_queued_item.is_none()
                    || last_seen_queued_item.is_some_and(|last| !self.is_newest(last))
                {
                    match self
                        .base_stream
                        .poll_next_unpin(&mut Context::from_waker(clone_waker))
                    {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(item) => {
                            trace!("The base stream was ready for clone {clone_id}.");
                            state = CloneTaskState::Woken {
                                last_seen: self.enqueue(clone_id, item.as_ref()),
                            };

                            Poll::Ready(item)
                        }
                    }
                } else {
                    trace!("The base stream was not ready for clone {clone_id}.");
                    match self.pop(clone_id).clone() {
                        TotalQueue::ItemPopped {
                            item,
                            index: peeked_index,
                        } => {
                            trace!(
                                "An item was popped from the queue of clone {clone_id} and it was \
                                 seen by all already."
                            );
                            *last_seen_queued_item = Some(peeked_index);
                            Poll::Ready(item)
                        }
                        TotalQueue::ItemCloned {
                            index: popped_index,
                            item,
                        } => {
                            trace!(
                                "An item was popped from the queue of clone {clone_id} and it was \
                                 not seen by all yet."
                            );
                            self.enqueue(clone_id, item.as_ref());
                            *last_seen_queued_item = Some(popped_index);
                            Poll::Ready(item.clone())
                        }
                        TotalQueue::Empty => {
                            trace!("The queue was empty for clone {clone_id}.");
                            Poll::Pending
                        }
                    }
                }
            }
        };

        self.clones.insert(clone_id, state);
        poll
    }
}
