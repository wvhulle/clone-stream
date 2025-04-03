mod bridge;
mod forked;
mod shared_bridge;
use bridge::ForkBridge;
pub use forked::ForkedStream;
use futures::Stream;
use shared_bridge::CloneableForkBridge;
/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    fn fork(self) -> ForkedStream<Self> {
        CloneableForkBridge::from(ForkBridge::from(self)).new_fork()
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
