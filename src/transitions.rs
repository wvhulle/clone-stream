use core::task::Context;
use std::task::{Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::{
    Fork,
    fork::{CloneState, SiblingClone},
    queue::TotalQueue,
};

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn handle_empty_queue(
        &mut self,
        clone: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(clone_waker))
        {
            Poll::Pending => {
                clone.state = CloneState::Sleeping;
                clone.waker = Some(clone_waker.clone());
                Poll::Pending
            }
            Poll::Ready(item) => {
                clone.state = CloneState::Unpolled;
                self.enqueue_new_item(clone.id, item.as_ref());
                Poll::Ready(item)
            }
        }
    }

    pub(crate) fn handle_sleeping_state(
        &mut self,
        clone: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        if let CloneState::Sleeping = clone.state {
            if !clone.waker.as_ref().unwrap().will_wake(clone_waker) {
                clone.waker = Some(clone_waker.clone());
            }
            if clone.last_seen.is_none()
                || clone
                    .last_seen
                    .is_some_and(|last| !self.latest_item_on_queue(last))
            {
                self.handle_empty_queue(clone, clone_waker)
            } else {
                trace!(
                    "There are still some remaining newer items in the queue that need to be \
                     dispatched to sleeping clones, while polling clone {}.",
                    clone.id
                );
                self.handle_woken_state(clone, clone_waker)
            }
        } else {
            unreachable!()
        }
    }

    pub(crate) fn handle_woken_state(
        &mut self,
        sibling: &mut SiblingClone,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self.pop_queue(sibling.id).clone() {
            TotalQueue::ItemPopped {
                item,
                index: peeked_index,
            } => {
                sibling.last_seen = Some(peeked_index);
                if !self.latest_item_on_queue(peeked_index) {
                    sibling.state = CloneState::Sleeping;
                    sibling.waker = Some(clone_waker.clone());
                }
                Poll::Ready(item)
            }
            TotalQueue::ItemCloned {
                index: popped_index,
                item,
            } => {
                sibling.last_seen = self.enqueue_new_item(sibling.id, item.as_ref());
                if !self.latest_item_on_queue(popped_index) {
                    sibling.state = CloneState::Sleeping;
                    sibling.waker = Some(clone_waker.clone());
                }
                Poll::Ready(item.clone())
            }
            TotalQueue::Empty => self.handle_empty_queue(sibling, clone_waker),
        }
    }
}
