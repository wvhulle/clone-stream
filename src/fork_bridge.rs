use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

#[derive(Default)]
pub struct ForkRef<Item> {
    pub waiting: usize,
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
            Some(item) => {
           fork.waiting =-1;
            Poll::Ready(item)},
            None => {
                
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(fork_waker))
                {
                    Poll::Pending => {
                        fork.waiting =+ 1;
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                    fork.waiting =- 1;
                        self.forks
                            .iter_mut()
                            .filter(|(other_fork, _)| fork_id != **other_fork)
                            .for_each(|(_, fork)| {
                                if fork.waiting > 0 {
                                    fork.items.push_back(item.clone());
                                }
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
