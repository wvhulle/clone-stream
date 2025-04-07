use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

pub struct ForkBridge<BaseStream>
where
    BaseStream: Stream,
{
    pub base_stream: Pin<Box<BaseStream>>,
    pub suspended_forks: BTreeMap<usize, (Option<Waker>, VecDeque<Option<BaseStream::Item>>)>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            suspended_forks: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.suspended_forks.clear();
    }

    pub fn poll(&mut self, fork_id: usize, fork_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        let fork = self.suspended_forks.get_mut(&fork_id).unwrap();
        match fork.1.pop_front() {
            Some(item) => Poll::Ready(item),
            None => match self
                .base_stream
                .poll_next_unpin(&mut Context::from_waker(fork_waker))
            {
                Poll::Pending => {
                    fork.0 = Some(fork_waker.clone());
                    Poll::Pending
                }
                Poll::Ready(item) => {
                    fork.0 = Some(fork_waker.clone());
                    self.suspended_forks
                        .iter_mut()
                        .filter(|(other_fork, _)| fork_id != **other_fork)
                        .for_each(|(_, (waker, queue))| {
                            if let Some(waker) = &waker {
                                queue.push_back(item.clone());
                                waker.wake_by_ref();
                            }
                        });
                    Poll::Ready(item)
                }
            },
        }
    }
}

impl<BaseStream> Deref for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Pin<Box<BaseStream>>;

    fn deref(&self) -> &Self::Target {
        &self.base_stream
    }
}
impl<BaseStream> DerefMut for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_stream
    }
}
