mod bridge;
mod clone;

use bridge::Bridge;
pub use clone::CloneStream;
use futures::Stream;

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream
/// that yields items of the same original item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Bridge::new(self))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
