mod receiver;
mod sender;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use forked_stream::{CloneStream, ForkStream};
use futures::task::Waker;
pub use receiver::Receiver;
pub use sender::Sender;

pub fn channel<Item>() -> (Sender<Item>, Receiver<Item>) {
    let channel_state = Arc::new(Mutex::new(ChannelState::default()));

    (
        Sender {
            channel_state: channel_state.clone(),
        },
        Receiver { channel_state },
    )
}

struct ChannelState<Item> {
    items_to_send: VecDeque<Item>,
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

pub struct Setup<T = ()>
where
    T: Clone,
{
    pub sender: Sender<T>,
    pub fork_0: CloneStream<Receiver<T>>,
    #[allow(dead_code)]
    pub fork_1: CloneStream<Receiver<T>>,
}

impl<T> Setup<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        let (tx, rx) = channel::<T>();
        let fork_0 = rx.fork();
        let fork_1 = fork_0.clone();
        Setup {
            sender: tx,
            fork_0,
            fork_1,
        }
    }
}
