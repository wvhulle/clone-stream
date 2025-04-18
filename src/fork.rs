use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::Fuse};
use log::trace;

#[derive(Default)]
pub enum CloneTaskState<Item> {
    #[default]
    NonActive,
    Active {
        waker: Waker,
        queue: VecDeque<Option<Item>>,
    },
}

impl<Item> CloneTaskState<Item> {
    pub fn n_queued_items(&self) -> usize {
        match self {
            CloneTaskState::NonActive => 0,
            CloneTaskState::Active { queue, .. } => queue.len(),
        }
    }

    pub fn active(&self) -> bool {
        match self {
            CloneTaskState::NonActive => false,
            CloneTaskState::Active { .. } => true,
        }
    }
}

pub(crate) struct Split<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub base_stream: Pin<Box<Fuse<BaseStream>>>,
    pub clones: BTreeMap<usize, CloneTaskState<BaseStream::Item>>,
}

impl<BaseStream> Split<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream.fuse()),
            clones: BTreeMap::default(),
        }
    }

    fn notify_sibling_clones(&mut self, current_clone_id: usize, item: Option<&BaseStream::Item>) {
        self.clones
            .iter_mut()
            .filter(|(id, _)| **id != current_clone_id)
            .for_each(|(other_clone_id, other_clone)| {
                if let CloneTaskState::Active { waker, queue } = other_clone {
                    trace!(
                        "Clone {current_clone_id} was polled. Its queue was empty. The input \
                         stream was polled. It yielded an item. Updating the queues of sibling \
                         clone {other_clone_id}."
                    );

                    queue.push_back(item.cloned());
                    waker.wake_by_ref();
                }
            });
    }

    pub(crate) fn update(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the split.");
        let state = self.clones.get_mut(&clone_id).unwrap();

        match state {
            CloneTaskState::Active { waker, queue } => {
                trace!("Clone {clone_id} is active already.");
                if waker.will_wake(clone_waker) {
                    trace!("An old waker was registered for clone {clone_id}.");

                    waker.clone_from(clone_waker);

                    match queue.pop_front() {
                        Some(item) => Poll::Ready(item),
                        None => match self
                            .base_stream
                            .poll_next_unpin(&mut Context::from_waker(clone_waker))
                        {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(item) => {
                                self.notify_sibling_clones(clone_id, item.as_ref());
                                Poll::Ready(item)
                            }
                        },
                    }
                } else {
                    trace!("No queue was present");
                    match self
                        .base_stream
                        .poll_next_unpin(&mut Context::from_waker(clone_waker))
                    {
                        Poll::Pending => {
                            waker.clone_from(clone_waker);
                            queue.clear();
                            Poll::Pending
                        }
                        Poll::Ready(item) => {
                            self.notify_sibling_clones(clone_id, item.as_ref());
                            Poll::Ready(item)
                        }
                    }
                }
            }
            CloneTaskState::NonActive => {
                trace!("No queue was present");
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(clone_waker))
                {
                    Poll::Pending => {
                        *state = CloneTaskState::Active {
                            waker: clone_waker.clone(),
                            queue: VecDeque::new(),
                        };
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        self.notify_sibling_clones(clone_id, item.as_ref());
                        Poll::Ready(item)
                    }
                }
            }
        }
    }
}
