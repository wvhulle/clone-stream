use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};

use futures::{Stream, task::Context};
use log::trace;

use super::ChannelState;

pub struct Receiver<Item> {
    pub(super) channel_state: Arc<Mutex<ChannelState<Item>>>,
}

impl<Item> Stream for Receiver<Item> {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut channel_state = self.channel_state.lock().unwrap();
        if let Some(cached_item) = channel_state.items_to_send.pop_front() {
            if let Some(sender_waker) = channel_state.sender_waiting.take() {
                sender_waker.wake();
            }

            Poll::Ready(Some(cached_item))
        } else {
            channel_state.receiver_waiting = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
