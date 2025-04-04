use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, stream::FusedStream};

use crate::shared_bridge::SharedBridge;

/// A wrapper around a stream that implements `Clone`.
pub struct ForkedStream<BaseStream>(SharedBridge<BaseStream>)
where
    BaseStream: Stream<Item: Clone>;

impl<BaseStream> From<SharedBridge<BaseStream>> for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: SharedBridge<BaseStream>) -> Self {
        ForkedStream(bridge)
    }
}

impl<BaseStream> Stream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Item = BaseStream::Item;

    fn poll_next(self: Pin<&mut Self>, new_context: &mut Context) -> Poll<Option<Self::Item>> {
        self.modify(|bridge| bridge.poll_base_stream(new_context.waker()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.get(|bridge| bridge.base_stream.size_hint())
    }
}

impl<BaseStream> FusedStream for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone> + FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.get(|bridge| bridge.base_stream.is_terminated())
    }
}

impl<BaseStream> Clone for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        ForkedStream(self.0.clone())
    }
}

impl<BaseStream> Deref for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = SharedBridge<BaseStream>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<BaseStream> DerefMut for ForkedStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
