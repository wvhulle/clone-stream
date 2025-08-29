//! # Lazily clone streams with `clone-stream`
//!
//! This module provides a way to fork a stream into multiple streams that can
//! be cloned and used independently.
//!
//! The [`CloneStream`] struct implements the
//! [`Stream`] trait and allows for cloning of the stream, while the [`Fork`]
//! struct manages the underlying "base" (or input) stream and the other sibling
//! stream clones.
//!
//! The [`ForkStream`] trait is implemented for any stream that yields items
//! that implement the `Clone` trait. This allows for easy conversion of a
//! stream into a [`CloneStream`]. Just import this trait if you want to use the
//! functionality in this library.
mod clone;
mod fork;

mod states;

pub use clone::CloneStream;
use fork::Fork;
use futures::Stream;

/// A trait that turns an input [`Stream`] with [`Stream::Item`]s that implement
/// [`Clone`] into a stream that is [`Clone`]. The output stream yields items of
/// the same type as the input stream.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use clone_stream::ForkStream;
    /// use futures::{FutureExt, StreamExt, stream};
    /// let non_clone_stream = stream::iter(0..10);
    /// let clone_stream = non_clone_stream.fork();
    /// let mut cloned_stream = clone_stream.clone();
    /// ```
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Fork::new(self))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}

impl<BaseStream> From<BaseStream> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    /// Forks the stream into a new stream that can be cloned.
    fn from(base_stream: BaseStream) -> CloneStream<BaseStream> {
        CloneStream::from(Fork::new(base_stream))
    }
}
