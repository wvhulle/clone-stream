use core::task::Context;
use std::task::{Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::{
    Fork,
    fork::{CloneState, SiblingClone},
    queue::QueuePopState,
};

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn fetch_input_item(
        &mut self,
        clone: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(clone_waker))
        {
            Poll::Pending => {
                clone.state = CloneState::Suspended;
                clone.waker = Some(clone_waker.clone());
                Poll::Pending
            }
            Poll::Ready(item) => {
                clone.state = CloneState::UpToDate;
                clone.waker = None;
                self.enqueue_new_item(clone.id, item.as_ref());
                Poll::Ready(item)
            }
        }
    }

    pub(crate) fn wake_up(
        &mut self,
        clone: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        if let CloneState::Suspended = clone.state {
            if !clone.waker.as_ref().unwrap().will_wake(clone_waker) {
                clone.waker = Some(clone_waker.clone());
            }
            match clone.last_seen {
                None => {
                    trace!(
                        "No items were pulled from the queue yet for clone {}.",
                        clone.id
                    );
                    // While polling this clone, the base stream was always ready immediately and
                    // there were no other clones sleeping.
                    self.fetch_input_item(clone, clone_waker)
                }
                Some(last) => {
                    // This clone has already been suspended at least once and an item was
                    // dispatched to this clone from another stream.
                    if self.latest_item_on_queue(last) {
                        // This clone already saw the latest item currently on the queue. We need to
                        // poll the input stream.
                        self.fetch_input_item(clone, clone_waker)
                    } else {
                        // This clone has not seen the latest item on the queue yet.
                        self.try_pop_queue(clone, clone_waker)
                    }
                }
            }
        } else {
            unreachable!()
        }
    }

    pub(crate) fn try_pop_queue(
        &mut self,
        sibling: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self.pop_queue(sibling.id).clone() {
            QueuePopState::ItemPopped {
                item,
                index: peeked_index,
            } => {
                sibling.last_seen = Some(peeked_index);
                if !self.latest_item_on_queue(peeked_index) {
                    sibling.state = CloneState::Suspended;
                    sibling.waker = Some(clone_waker.clone());
                }
                Poll::Ready(item)
            }
            QueuePopState::ItemCloned {
                index: popped_index,
                item,
            } => {
                sibling.last_seen = self.enqueue_new_item(sibling.id, item.as_ref());
                if !self.latest_item_on_queue(popped_index) {
                    sibling.state = CloneState::Suspended;
                    sibling.waker = Some(clone_waker.clone());
                }
                Poll::Ready(item.clone())
            }
            QueuePopState::Empty => self.fetch_input_item(sibling, clone_waker),
        }
    }
}
