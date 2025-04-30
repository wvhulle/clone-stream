use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt, future::FusedFuture, stream::FusedStream};

use crate::CloneStream;

pub struct Next<'clone, St>
where
    St: Stream<Item: Clone>,
{
    clone_stream: &'clone mut CloneStream<St>,
}

impl<St> Future for Next<'_, St>
where
    St: Stream<Item: Clone> + Unpin,
{
    type Output = Option<St::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.clone_stream.poll_next_unpin(cx)
    }
}

impl<St> Unpin for Next<'_, St> where St: Stream<Item: Clone> + Sized + Unpin {}

impl<'a, St> Next<'a, St>
where
    St: Stream<Item: Clone>,
{
    pub(super) fn new(clone_stream: &'a mut CloneStream<St>) -> Self {
        Self { clone_stream }
    }
}

impl<St> FusedFuture for Next<'_, St>
where
    St: Stream<Item: Clone> + FusedStream + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.clone_stream.is_terminated()
    }
}

impl<St> Drop for Next<'_, St>
where
    St: Stream<Item: Clone>,
{
    fn drop(&mut self) {
        let mut split = self.clone_stream.split.write().unwrap();
        split.cancel_pending(self.clone_stream.id);
    }
}
