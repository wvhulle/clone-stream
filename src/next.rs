use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt, future::FusedFuture, stream::FusedStream};

use crate::CloneStream;

pub struct MyNext<'a, St>
where
    St: Stream<Item: Clone>,
{
    stream: &'a mut CloneStream<St>,
}

impl<St: Unpin> Unpin for MyNext<'_, St> where St: Stream<Item: Clone> + Sized {}

impl<'a, St: Stream<Item: Clone>> MyNext<'a, St> {
    pub(super) fn new(stream: &'a mut CloneStream<St>) -> Self {
        Self { stream }
    }
}

impl<St> FusedFuture for MyNext<'_, St>
where
    St: Stream<Item: Clone> + FusedStream + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<St> Future for MyNext<'_, St>
where
    St: Stream<Item: Clone> + Unpin,
{
    type Output = Option<St::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.stream.poll_next_unpin(cx)
    }
}

impl<S> Drop for MyNext<'_, S>
where
    S: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut split = self.stream.split.write().unwrap();
        split.cancel(self.stream.id);
    }
}
