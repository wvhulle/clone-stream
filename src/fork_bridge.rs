use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

#[derive(Default)]
pub struct ForkRef<Item> {
    pub wakers: VecDeque<Waker>,
    pub items: VecDeque<Item>,
}

pub struct ForkBridge<BaseStream>
where
    BaseStream: Stream,
{
    pub base_stream: Pin<Box<BaseStream>>,
    pub forks: BTreeMap<usize, ForkRef<Option<BaseStream::Item>>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            forks: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.forks.clear();
    }

    pub fn poll(&mut self, fork_id: usize, fork_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        let fork = self.forks.get_mut(&fork_id).unwrap();

        match fork.items.pop_front() {
            Some(item) => Poll::Ready(item),
            None => {
                fork.wakers.retain(|o| !o.will_wake(fork_waker));
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(fork_waker))
                {
                    Poll::Pending => {
                        fork.wakers.push_back(fork_waker.clone());
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        self.forks
                            .iter_mut()
                            .filter(|(other_fork, _)| fork_id != **other_fork)
                            .for_each(|(_, fork)| {
                                if !fork.wakers.is_empty() {
                                    fork.items.push_back(item.clone());
                                }
                                fork.wakers
                                    .iter()
                                    .filter(|w| !w.will_wake(fork_waker))
                                    .for_each(Waker::wake_by_ref);
                            });
                        Poll::Ready(item)
                    }
                }
            }
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
