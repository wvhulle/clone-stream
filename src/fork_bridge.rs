use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

use crate::task_item_queue::SuspendedForks;

pub struct ForkBridge<BaseStream>
where
    BaseStream: Stream,
{
    pub base_stream: Pin<Box<BaseStream>>,
    pub suspended_forks: SuspendedForks<Option<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(base_stream: BaseStream, max_items_cached: Option<usize>) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            suspended_forks: SuspendedForks::new(max_items_cached),
        }
    }

    pub fn clear(&mut self) {
        self.suspended_forks.clear();
    }

    pub fn poll(&mut self, fork_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        println!("Polling the bridge with waker: {:?}", fork_waker.data());
        if let Some(item) = self.suspended_forks.earliest_item(fork_waker) {
            println!("The passed waker has an associated item still waiting.");
            //self.suspended_forks.forget_if_queue_empty(fork_waker);
            Poll::Ready(item)
        } else {
            println!("The passed waker does not have an associated item waiting.");
            match self
                .base_stream
                .poll_next_unpin(&mut Context::from_waker(fork_waker))
            {
                Poll::Pending => {
                    println!("There is no next item on the input stream.");
                    self.suspended_forks.insert_empty_queue(fork_waker.clone());
                    Poll::Pending
                }
                Poll::Ready(item) => {
                    println!("Got a next item from the input stream.");
                    self.suspended_forks.append(item.clone(), fork_waker);
                    self.suspended_forks.wake_all();
                    Poll::Ready(item)
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
