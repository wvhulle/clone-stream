use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, stream::FusedStream};

use crate::shared_bridge::SharedBridge;

/// A stream that implements `Clone` and takes input from the `BaseStream`.
pub struct ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    bridge: SharedBridge<BaseStream>,
    waker: Option<Waker>,
}

impl<BaseStream> From<SharedBridge<BaseStream>> for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: SharedBridge<BaseStream>) -> Self {
        ForkedStream {
            bridge,
            waker: None,
        }
    }
}

impl<BaseStream> Stream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Item = BaseStream::Item;

    fn poll_next(mut self: Pin<&mut Self>, fork_task_cx: &mut Context) -> Poll<Option<Self::Item>> {
        let poll_result = self.modify(|bridge| bridge.poll(fork_task_cx.waker()));

        if poll_result.is_pending() {
            self.waker = Some(fork_task_cx.waker().clone());
        } else {
            self.waker = None;
        }

        poll_result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.waker {
            Some(waker) => self.get(|bridge| {
                let (lower, upper) = bridge.base_stream.size_hint();
                let n_cached = bridge.suspended_forks.n_cached(waker);
                (lower + n_cached, upper.map(|u| u + n_cached))
            }),
            None => self.get(|bridge| bridge.base_stream.size_hint()),
        }
    }
}

impl<BaseStream> FusedStream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone> + FusedStream,
{
    fn is_terminated(&self) -> bool {
        match &self.waker {
            Some(waker) => self.get(|bridge| {
                bridge.base_stream.is_terminated() && bridge.suspended_forks.n_cached(waker) == 0
            }),
            None => self.get(|bridge| bridge.base_stream.is_terminated()),
        }
    }
}

impl<BaseStream> Clone for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        Self::from(self.bridge.clone())
    }
}

impl<BaseStream> Deref for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = SharedBridge<BaseStream>;

    fn deref(&self) -> &Self::Target {
        &self.bridge
    }
}

impl<BaseStream> DerefMut for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bridge
    }
}
