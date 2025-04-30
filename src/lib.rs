//! # Clone streams with `clone-stream`
//!
//! This module provides a way to fork a stream into multiple streams that can
//! be cloned and used independently. The `CloneStream` struct implements the
//! `Stream` trait and allows for cloning of the stream, while the `Fork` struct
//! manages the underlying stream and its clones.
//!
//! The [`ForkStream`] trait is implemented for any stream that yields items
//! that implement the `Clone` trait. This allows for easy conversion of a
//! stream into a [`CloneStream`].
mod clone;
mod fork;
mod next;
mod states;

pub use clone::CloneStream;
use fork::Fork;
use futures::Stream;

impl<BaseStream> From<BaseStream> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    /// Forks the stream into a new stream that can be cloned.
    fn from(base_stream: BaseStream) -> CloneStream<BaseStream> {
        CloneStream::from(Fork::new(base_stream))
    }
}

/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream
/// that yields items of the same original item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use clone_stream::ForkStream;
    /// use futures::{FutureExt, StreamExt, stream};
    /// let uncloneable_stream = stream::iter(0..10);
    /// let cloneable_stream = uncloneable_stream.fork();
    /// let mut cloned_stream = cloneable_stream.clone();
    /// ```
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Fork::new(self))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
