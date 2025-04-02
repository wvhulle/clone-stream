mod receiver;
mod sender;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use futures::task::Waker;
pub use receiver::Receiver;
pub use sender::Sender;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let channel_state = Arc::new(Mutex::new(Default::default()));

    (
        Sender {
            channel_state: channel_state.clone(),
        },
        Receiver { channel_state },
    )
}

struct ChannelState<T> {
    items_to_send: VecDeque<T>,
    sender_waiting: Option<Waker>,
    receiver_waiting: Option<Waker>,
}

impl<Item> Default for ChannelState<Item> {
    fn default() -> Self {
        Self {
            items_to_send: VecDeque::new(),
            sender_waiting: None,
            receiver_waiting: None,
        }
    }
}
