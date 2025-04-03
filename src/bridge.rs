use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

pub struct ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(super) stream: Pin<Box<BaseStream>>,
    pub(super) waiters: VecDeque<Waker>,
    pub(super) last_input: Poll<Option<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn add_waker(&mut self, waker: Waker) {
        self.remove_waker(&waker);
        self.waiters.push_back(waker);
    }

    pub fn remove_waker(&mut self, waker: &Waker) {
        self.waiters.retain(|w| !w.will_wake(waker));
    }

    pub(super) fn handle_fork(&mut self, current_task: &Waker) -> Poll<Option<BaseStream::Item>> {
        let poll = self
            .stream
            .poll_next_unpin(&mut Context::from_waker(current_task));
        match poll {
            Poll::Ready(maybe_item) => {
                self.last_input = Poll::Ready(maybe_item.clone());

                self.waiters.clone().into_iter().for_each(|waker| {
                    waker.wake_by_ref();
                });

                Poll::Ready(maybe_item)
            }
            Poll::Pending => match self.last_input.clone() {
                Poll::Pending => {
                    self.add_waker(current_task.clone());
                    Poll::Pending
                }
                Poll::Ready(item) => {
                    self.remove_waker(current_task);
                    if self.waiters.is_empty() {
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
            stream: Box::pin(stream),
            waiters: VecDeque::new(),
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
        &self.stream
    }
}
impl<BaseStream> DerefMut for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}
