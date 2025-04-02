mod bridge;
mod forked;
mod mock;

use bridge::ForkBridge;
pub use forked::ForkedStream;
use futures::Stream;
pub use mock::{
    SpscSender, channel as spsc_channel, log_init, new_concurrent_setup,
    new_sender_and_shared_stream,
};

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    fn fork(self) -> ForkedStream<Self> {
        ForkBridge::from(self).into()
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
