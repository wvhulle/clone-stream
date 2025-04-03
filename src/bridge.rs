use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

pub struct ForkBridge<BaseStream>
where
    BaseStream: Stream,
{
    pub(super) base_stream: Pin<Box<BaseStream>>,
    pub(super) suspended_forks: VecDeque<Waker>,
    pub(super) last_input: Poll<Option<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream,
{
    pub fn add_waker(&mut self, waker: Waker) {
        self.remove_waker(&waker);
        self.suspended_forks.push_back(waker);
    }

    pub fn remove_waker(&mut self, waker: &Waker) {
        self.suspended_forks.retain(|w| !w.will_wake(waker));
    }
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn poll_base_stream(&mut self, fork_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        match self
            .base_stream
            .poll_next_unpin(&mut Context::from_waker(fork_waker))
        {
            Poll::Ready(item) => {
                self.last_input = Poll::Ready(item.clone());

                self.suspended_forks
                    .clone()
                    .into_iter()
                    .for_each(|other_fork| {
                        other_fork.wake_by_ref();
                    });

                Poll::Ready(item)
            }
            Poll::Pending => match self.last_input.clone() {
                Poll::Pending => {
                    self.add_waker(fork_waker.clone());
                    Poll::Pending
                }
                Poll::Ready(item) => {
                    self.remove_waker(fork_waker);
                    if self.suspended_forks.is_empty() {
                        self.last_input = Poll::Pending;
                    }
                    Poll::Ready(item.clone())
                }
            },
        }
    }
}

impl<BaseStream> From<BaseStream> for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(stream),
            suspended_forks: VecDeque::new(),
            last_input: Poll::Pending,
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
