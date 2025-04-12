use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{
    Stream, StreamExt,
    stream::{Fuse, FusedStream},
};
use log::{debug, trace};

#[derive(Default)]
pub enum TaskState {
    #[default]
    NonActive,
    Active(Waker),
    Terminated,
}

impl TaskState {
    pub fn active(&self) -> bool {
        matches!(self, TaskState::Active(_))
    }
}
pub struct UnseenByClone<Item> {
    pub(crate) suspended_task: TaskState,
    pub(crate) unseen_items: VecDeque<Item>,
}

impl<Item> Default for UnseenByClone<Item> {
    fn default() -> Self {
        Self {
            suspended_task: TaskState::NonActive,
            unseen_items: VecDeque::new(),
        }
    }
}

pub(crate) struct Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub base_stream: Pin<Box<Fuse<BaseStream>>>,
    pub clones: BTreeMap<usize, UnseenByClone<BaseStream::Item>>,
}

impl<BaseStream> Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream.fuse()),
            clones: BTreeMap::default(),
        }
    }

    pub(crate) fn poll(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the bridge.");
        let clone = self.clones.get_mut(&clone_id).unwrap();

        match clone.unseen_items.pop_front() {
            Some(item) => {
                debug!("Popped an item from the queue of {clone_id}.");
                clone.suspended_task = TaskState::Active(clone_waker.clone());
                Poll::Ready(Some(item))
            }
            None => {
                if self.base_stream.is_terminated() {
                    debug!(
                        "The input stream is terminated and items are on the queue of clone \
                         {clone_id}. Marking clone as terminated."
                    );
                    clone.suspended_task = TaskState::Terminated;
                    Poll::Ready(None)
                } else {
                    match &self
                        .base_stream
                        .poll_next_unpin(&mut Context::from_waker(clone_waker))
                    {
                        Poll::Pending => {
                            debug!(
                                "No ready item from input stream available for clone {clone_id}"
                            );
                            clone.suspended_task = TaskState::Active(clone_waker.clone());
                            Poll::Pending
                        }
                        poll @ Poll::Ready(item) => {
                            self.terminate_or_wake_all(item.clone(), clone_id, clone_waker);
                            poll.clone()
                        }
                    }
                }
            }
        }
    }

    fn terminate_or_wake_all(
        &mut self,
        item: Option<BaseStream::Item>,
        clone_id: usize,
        clone_waker: &Waker,
    ) {
        let clone = self.clones.get_mut(&clone_id).unwrap();
        if let Some(item) = item {
            trace!("While polling {clone_id}, the input stream yield a Some.");
            clone.suspended_task = TaskState::Active(clone_waker.clone());

            self.clones
                .iter_mut()
                .filter(|(other_clone, _)| clone_id != **other_clone)
                .for_each(|(other_clone_id, other_clone)| {
                    if let TaskState::Active(waker) = &other_clone.suspended_task {
                        debug!(
                            "The item will be added to the queue of another active fork \
                             {other_clone_id} and it will be woken up."
                        );
                        other_clone.unseen_items.push_back(item.clone());
                        waker.wake_by_ref();
                    } else {
                        trace!(
                            "The other fork {other_clone_id} is not active and the item will not \
                             be added to its queue."
                        );
                    }
                });
        } else {
            debug!(
                "While polling clone {clone_id}, the input stream yielded a None. Marking this \
                 fork as terminated."
            );
            clone.suspended_task = TaskState::Terminated;
        }
    }
}
