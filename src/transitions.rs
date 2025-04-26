use core::task::Context;
use std::task::{Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::{Fork, queue::QueuePopState};

#[derive(Debug, Clone)]
pub(crate) struct OutputStatePoll<T> {
    pub(crate) state: CloneState,
    pub(crate) poll_result: Poll<T>,
}

#[derive(Default, Debug, Clone)]
pub(crate) enum CloneState {
    #[default]
    UpToDate,
    ReadyToPop(ReadyToPop),
    Suspended(Suspended),
}

impl CloneState {
    pub(crate) fn older_than(&self, item_index: usize) -> bool {
        match self {
            CloneState::Suspended(Suspended { last_seen, .. }) => {
                last_seen.is_none() || last_seen.is_some_and(|last| last < item_index)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReadyToPop {
    pub(crate) last_seen: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct Suspended {
    pub(crate) last_seen: Option<usize>,
    pub(crate) waker: Waker,
}

impl Suspended {
    pub(crate) fn wake_up<BaseStream>(
        &self,
        clone_waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> OutputStatePoll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match self.last_seen {
            None => {
                // While polling this clone, the base stream was always ready immediately and
                // there were no other clones sleeping.
                fork.fetch_input_item(clone_waker)
            }
            Some(last) => {
                // This clone has already been suspended at least once and an item was
                // dispatched to this clone from another stream.
                if fork.latest_item_index(last) {
                    // This clone already saw the latest item currently on the queue. We need to
                    // poll the input stream.
                    fork.fetch_input_item(clone_waker)
                } else {
                    // This clone has not seen the latest item on the queue yet.
                    fork.try_pop_queue(clone_waker)
                }
            }
        }
    }
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn fetch_input_item(
        &mut self,
        clone_waker: &Waker,
    ) -> OutputStatePoll<Option<BaseStream::Item>> {
        trace!("Base stream is being polled.");
        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(clone_waker))
        {
            Poll::Pending => OutputStatePoll {
                state: CloneState::Suspended(Suspended {
                    last_seen: None,
                    waker: clone_waker.clone(),
                }),

                poll_result: Poll::Pending,
            },
            Poll::Ready(item) => {
                self.enqueue_new_item(item.as_ref());
                OutputStatePoll {
                    state: CloneState::UpToDate,
                    poll_result: Poll::Ready(item),
                }
            }
        }
    }
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn try_pop_queue(
        &mut self,
        clone_waker: &Waker,
    ) -> OutputStatePoll<Option<BaseStream::Item>> {
        match self.pop_queue().clone() {
            QueuePopState::ItemPopped {
                item,
                index: peeked_index,
            } => {
                let state = if self.latest_item_index(peeked_index) {
                    CloneState::UpToDate
                } else {
                    CloneState::Suspended(Suspended {
                        waker: clone_waker.clone(),
                        last_seen: Some(peeked_index),
                    })
                };
                OutputStatePoll {
                    poll_result: Poll::Ready(item),
                    state,
                }
            }
            QueuePopState::ItemCloned {
                index: popped_index,
                item,
            } => {
                let new_index = self.enqueue_new_item(item.as_ref());
                let state = if self.latest_item_index(popped_index) {
                    CloneState::UpToDate
                } else {
                    CloneState::Suspended(Suspended {
                        waker: clone_waker.clone(),
                        last_seen: new_index,
                    })
                };

                OutputStatePoll {
                    state,
                    poll_result: Poll::Ready(item.clone()),
                }
            }
            QueuePopState::Empty => self.fetch_input_item(clone_waker),
        }
    }
}
