mod bridge;
mod stream;

use bridge::ForkBridge;
pub use stream::ForkedStream;
use futures::Stream;

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    /// The `max_buffered` parameter controls how many items can be buffered for each fork.
    fn fork(self) -> ForkedStream<Self> {
        ForkedStream::new(ForkBridge::new(self))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
