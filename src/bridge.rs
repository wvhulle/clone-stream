use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::Fuse};
use log::trace;

// #[derive(Default)]
pub enum CloneTaskState<Item> {
    NonActive,
    Active(VecDeque<(Waker, VecDeque<Option<Item>>)>),
}

impl<Item> CloneTaskState<Item> {
    pub fn max_size(&self) -> usize {
        match self {
            CloneTaskState::NonActive => 0,
            CloneTaskState::Active(wakers) => wakers.len(),
        }
    }

    pub fn active(&self) -> bool {
        match self {
            CloneTaskState::NonActive => false,
            CloneTaskState::Active(_) => true,
        }
    }
}

pub struct UnseenByClone<Item> {
    pub(crate) state: CloneTaskState<Item>,
}

impl<Item> Default for UnseenByClone<Item> {
    fn default() -> Self {
        Self {
            state: CloneTaskState::NonActive,
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
    #[allow(clippy::too_many_lines)]
    pub(crate) fn poll(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the bridge.");
        let clone = self.clones.get_mut(&clone_id).unwrap();

        match &mut clone.state {
            CloneTaskState::Active(wakers) => {
                trace!("Clone {clone_id} is active already.");
                if let Some((old_waker, queue)) = wakers
                    .iter_mut()
                    .find(|(old_waker, _)| old_waker.will_wake(clone_waker))
                {
                    trace!("An old waker was registered for clone {clone_id}.");

                    old_waker.clone_from(clone_waker);

                    match queue.pop_front() {
                        Some(item) => {
                            match &item {
                                Some(_) => {
                                    trace!(
                                        "The queue of clone {clone_id} was not empty. Returning a \
                                         non-null
                                         item from the queue."
                                    );
                                }
                                None => {
                                    trace!(
                                        "The queue of clone {clone_id} was empty. Returning None."
                                    );
                                }
                            }
                            Poll::Ready(item)
                        }
                        None => match self
                            .base_stream
                            .poll_next_unpin(&mut Context::from_waker(clone_waker))
                        {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(item) => {
                                self.clones
                                    .iter_mut()
                                    .filter(|(id, _)| **id != clone_id)
                                    .for_each(|(other_clone_id, other_clone)| {
                                        if let CloneTaskState::Active(wakers) =
                                            &mut other_clone.state
                                        {
                                            trace!(
                                                "Clone {clone_id} was polled. Its queue was \
                                                 empty. The input stream was polled. It yielded \
                                                 an item. Updating the queues of sibling clone \
                                                 {other_clone_id}."
                                            );
                                            for (old_waker, queue) in wakers.iter_mut() {
                                                queue.push_back(item.clone());
                                                old_waker.wake_by_ref();
                                            }
                                        }
                                    });
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
                            wakers.push_back((clone_waker.clone(), VecDeque::new()));
                            Poll::Pending
                        }
                        Poll::Ready(item) => {
                            self.clones
                                .iter_mut()
                                .filter(|(id, _)| **id != clone_id)
                                .for_each(|(other_clone_id, other_clone)| {
                                    if let CloneTaskState::Active(wakers) = &mut other_clone.state {
                                        trace!(
                                            "Clone {clone_id} was polled. Its queue was empty. \
                                             The input stream was polled. It yielded an item. \
                                             Updating the queues of sibling clone \
                                             {other_clone_id}."
                                        );
                                        for (old_waker, queue) in wakers.iter_mut() {
                                            queue.push_back(item.clone());
                                            old_waker.wake_by_ref();
                                        }
                                    }
                                });
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
                        clone.state = CloneTaskState::Active(VecDeque::from([(
                            clone_waker.clone(),
                            VecDeque::new(),
                        )]));
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        self.clones
                            .iter_mut()
                            .filter(|(id, _)| **id != clone_id)
                            .for_each(|(other_clone_id, other_clone)| {
                                if let CloneTaskState::Active(wakers) = &mut other_clone.state {
                                    trace!(
                                        "Clone {clone_id} was polled. Its queue was empty. The \
                                         input stream was polled. It yielded an item. Updating \
                                         the queues of sibling clone {other_clone_id}."
                                    );
                                    for (old_waker, queue) in wakers.iter_mut() {
                                        queue.push_back(item.clone());
                                        old_waker.wake_by_ref();
                                    }
                                }
                            });
                        Poll::Ready(item)
                    }
                }
            }
        }
    }
}
