use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::Fuse};
use log::trace;

#[derive(Default)]
pub enum TaskState<Item> {
    #[default]
    NonActive,
    Active(VecDeque<(Waker, VecDeque<Item>)>),
    Terminated,
}

impl<Item> TaskState<Item> {
    pub fn active(&self) -> bool {
        matches!(self, TaskState::Active(_))
    }
}
pub struct UnseenByClone<Item> {
    pub(crate) state: TaskState<Item>,
}

impl<Item> Default for UnseenByClone<Item> {
    fn default() -> Self {
        Self {
            state: TaskState::NonActive,
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

        match &mut clone.state {
            TaskState::Active(wakers) => {
                trace!("Clone {clone_id} is active already.");
                if let Some((old_waker, queue)) = wakers
                    .iter_mut()
                    .find(|(old_waker, _)| old_waker.will_wake(clone_waker))
                {
                    trace!("An old waker was registered for clone {clone_id}.");

                    old_waker.clone_from(clone_waker);
                    match queue.pop_front() {
                        Some(item) => Poll::Ready(Some(item)),
                        None => self.query_input(clone_id, clone_waker),
                    }
                } else {
                    trace!("Clone {clone_id} was not polled before from this waker.");
                    wakers.push_back((clone_waker.clone(), VecDeque::new()));
                    self.query_input(clone_id, clone_waker)
                }
            }
            TaskState::NonActive => {
                trace!("Clone {clone_id} was not polled before.");
                clone.state =
                    TaskState::Active(VecDeque::from([(clone_waker.clone(), VecDeque::new())]));
                self.query_input(clone_id, clone_waker)
            }
            TaskState::Terminated => {
                trace!("Clone {clone_id} has been terminated.");
                Poll::Ready(None)
            }
        }
    }
    #[allow(clippy::too_many_lines)]
    fn query_input(&mut self, clone_id: usize, waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        let clone = self.clones.get_mut(&clone_id).unwrap();

        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(waker))
        {
            Poll::Pending => {
                trace!(
                    "Clone {clone_id} was polled and afterwards the input stream. The input \
                     stream is pending. Updating the clone's state."
                );
                match &mut clone.state {
                    TaskState::NonActive => {
                        trace!(
                            "Clone {clone_id} was not active. Initialising it with a waker and \
                             empty item queue."
                        );
                        clone.state =
                            TaskState::Active(VecDeque::from([(waker.clone(), VecDeque::new())]));
                        Poll::Pending
                    }
                    TaskState::Active(wakers) => {
                        if let Some((old_waker, _)) = wakers
                            .iter_mut()
                            .find(|(old_waker, _)| old_waker.will_wake(waker))
                        {
                            trace!(
                                "An old waker was registered for clone {clone_id}. Updating the \
                                 old waker to the new waker."
                            );
                            old_waker.clone_from(waker);
                        } else {
                            trace!(
                                "Clone {clone_id} was not polled before from this waker. \
                                 Initialising an empty item queue."
                            );
                            wakers.push_back((waker.clone(), VecDeque::new()));
                        }
                        Poll::Pending
                    }
                    TaskState::Terminated => Poll::Ready(None),
                }
            }
            Poll::Ready(item) => {
                if let Some(item) = item {
                    match &mut clone.state {
                        TaskState::NonActive => {
                            trace!(
                                "Clone {clone_id} was not active. Initialising it with a waker \
                                 and empty item queue."
                            );
                            clone.state = TaskState::Active(VecDeque::from([(
                                waker.clone(),
                                VecDeque::new(),
                            )]));
                        }
                        TaskState::Active(wakers) => {
                            trace!(
                                "Clone {clone_id} was polled and afterwards the input stream. The \
                                 input stream is ready now. Updating the clone's state."
                            );
                            if let Some((old_waker, _)) = wakers
                                .iter_mut()
                                .find(|(old_waker, _)| old_waker.will_wake(waker))
                            {
                                trace!(
                                    "An old waker was registered for clone {clone_id}. Updating \
                                     the old waker to the new waker."
                                );
                                old_waker.clone_from(waker);
                            }

                            wakers
                                .iter_mut()
                                .filter(|(old_waker, _)| !old_waker.will_wake(waker))
                                .for_each(|(other_waker, queue)| {
                                    trace!(
                                        "Adding item to queue of another waker for clone \
                                         {clone_id}."
                                    );
                                    queue.push_back(item.clone());
                                    other_waker.wake_by_ref();
                                });
                        }
                        TaskState::Terminated => {}
                    }

                    self.clones
                        .iter_mut()
                        .filter(|(id, _)| **id != clone_id)
                        .for_each(|(other_clone_id, clone)| {
                            if let TaskState::Active(wakers) = &mut clone.state {
                                trace!(
                                    "Clone {clone_id} was polled. Its queue was empty. The input \
                                     stream was polled. It yielded an item. Updating the queues \
                                     of sibling clone {other_clone_id}."
                                );
                                for (old_waker, queue) in wakers.iter_mut() {
                                    queue.push_back(item.clone());
                                    old_waker.wake_by_ref();
                                }
                            }
                        });
                    Poll::Ready(Some(item))
                } else {
                    match &mut clone.state {
                        TaskState::NonActive => clone.state = TaskState::Terminated,
                        TaskState::Active(items) => {
                            items.retain(|(old_waker, _)| !old_waker.will_wake(waker));
                        }
                        TaskState::Terminated => (),
                    }
                    Poll::Ready(None)
                }
            }
        }
    }
}
