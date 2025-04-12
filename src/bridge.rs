use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{
    Stream, StreamExt,
    stream::{Fuse, FusedStream},
};
use log::trace;

#[derive(Default)]
pub enum TaskState {
    #[default]
    NonActive,
    Active(Waker),
}

impl TaskState {
    pub fn active(&self) -> bool {
        matches!(self, TaskState::Active(_))
    }
}
pub struct UnseenByClone<Item> {
    pub suspended_task: TaskState,
    pub unseen_items: VecDeque<Item>,
}

impl<Item> Default for UnseenByClone<Item> {
    fn default() -> Self {
        Self {
            suspended_task: TaskState::NonActive,
            unseen_items: VecDeque::new(),
        }
    }
}

pub struct Bridge<BaseStream>
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
    pub fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream.fuse()),
            clones: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.clones.clear();
    }

    pub fn poll(&mut self, clone_id: usize, clone_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the bridge.");
        let clone = self.clones.get_mut(&clone_id).unwrap();

        clone.suspended_task = TaskState::Active(clone_waker.clone());

        match clone.unseen_items.pop_front() {
            Some(item) => {
                trace!("Popping item for clone {clone_id} from queue");
                Poll::Ready(Some(item))
            }
            None => {
                if self.base_stream.is_terminated() {
                    Poll::Ready(None)
                } else {
                    match self
                        .base_stream
                        .poll_next_unpin(&mut Context::from_waker(clone_waker))
                    {
                        Poll::Pending => {
                            trace!(
                                "No ready item from input stream available for clone {clone_id}"
                            );
                            Poll::Pending
                        }
                        Poll::Ready(item) => {
                            if let Some(item) = item {
                                trace!("Item ready from input stream for clone {clone_id}");
                                self.clones
                                    .iter_mut()
                                    .filter(|(other_clone, _)| clone_id != **other_clone)
                                    .for_each(|(other_clone_id, other_clone)| {
                                        if let TaskState::Active(waker) =
                                            &other_clone.suspended_task
                                        {
                                            trace!(
                                                "Waking up clone {other_clone_id} because it was \
                                                 polled"
                                            );
                                            other_clone.unseen_items.push_back(item.clone());
                                            waker.wake_by_ref();
                                        }
                                    });

                                Poll::Ready(Some(item))
                            } else {
                                trace!("Input stream is terminated");
                                Poll::Ready(None)
                            }
                        }
                    }
                }
            }
        }
    }
}
