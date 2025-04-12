use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};

use futures::{Sink, task::Context};

use super::ChannelState;

pub struct Sender<Item> {
    pub(super) channel_state: Arc<Mutex<ChannelState<Item>>>,
}

impl<Item> Sink<Item> for Sender<Item> {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut channel_state = self.channel_state.lock().unwrap();
        if channel_state.receiver_waiting.is_some() {
            Poll::Ready(Ok(()))
        } else {
            // Receiver not actively polling, sender must wait.
            channel_state.sender_waiting = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let mut channel_state = self.channel_state.lock().unwrap();
        channel_state.items_to_send.push_back(item);
        if let Some(waker) = channel_state.receiver_waiting.take() {
            waker.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
