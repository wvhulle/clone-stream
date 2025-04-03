mod bridge;
mod buffer;
mod forked;
mod shared_bridge;
use bridge::ForkBridge;
pub use forked::ForkedStream;
use futures::Stream;
use shared_bridge::CloneableForkBridge;

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    /// The `max_buffered` parameter controls how many items can be buffered for each fork.
    fn fork(self, max_buffered: impl Into<Option<usize>>) -> ForkedStream<Self> {
        CloneableForkBridge::from(ForkBridge::new(self, max_buffered.into())).into()
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
