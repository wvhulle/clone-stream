mod fork_bridge;
mod forked_stream;
mod shared_bridge;
mod task_item_queue;
use fork_bridge::ForkBridge;
pub use forked_stream::ForkedStream;
use futures::Stream;
use shared_bridge::SharedBridge;

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    /// The `max_buffered` parameter controls how many items can be buffered for each fork.
    fn fork(self, max_items_cached: impl Into<Option<usize>>) -> ForkedStream<Self> {
        SharedBridge::from(ForkBridge::new(self, max_items_cached.into())).into()
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
