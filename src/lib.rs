//! # Clone streams with `clone-stream`
//!
//! Turn any [`Stream`] into a cloneable stream where each clone receives all
//! items independently.
//!
//! The [`CloneStream`] struct implements [`Clone`] + [`Stream`], allowing you
//! to create multiple independent consumers of the same data. The
//! [`ForkStream`] trait provides the entry point for converting regular
//! streams.
//!
//! # Quick Start
//!
//! ```rust
//! use clone_stream::ForkStream;
//! use futures::{StreamExt, stream};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let stream = stream::iter(vec![1, 2, 3]).fork();
//! let mut clone1 = stream.clone();
//! let mut clone2 = stream.clone();
//! // Both clones receive all items independently
//! # }
//! ```
mod clone;
mod error;
mod fork;
mod ring_queue;
mod states;

pub use clone::CloneStream;
pub use error::{CloneStreamError, Result};
use fork::Fork;
pub use fork::ForkConfig;
use futures::Stream;

/// Extension trait to make any [`Stream`] cloneable.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Creates a cloneable version of this stream.
    ///
    /// ```rust
    /// use clone_stream::ForkStream;
    /// use futures::{StreamExt, stream};
    ///
    /// let stream = stream::iter(0..3).fork();
    /// let mut clone = stream.clone();
    /// ```
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Fork::new(self))
    }

    /// Creates a cloneable stream with custom limits.
    ///
    /// # Arguments
    /// * `max_queue_size` - Max items queued before panic
    /// * `max_clone_count` - Max clones before panic
    ///
    /// # Panics
    /// When limits are exceeded during operation.
    ///
    /// ```rust
    /// use clone_stream::ForkStream;
    /// use futures::stream;
    ///
    /// let stream = stream::iter(0..3).fork_with_limits(100, 5);
    /// ```
    fn fork_with_limits(self, max_queue_size: usize, max_clone_count: usize) -> CloneStream<Self> {
        let config = ForkConfig {
            max_clone_count,
            max_queue_size,
        };
        CloneStream::from(Fork::with_config(self, config))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}

impl<BaseStream> From<BaseStream> for CloneStream<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    /// Converts a stream into a cloneable stream.
    fn from(base_stream: BaseStream) -> CloneStream<BaseStream> {
        CloneStream::from(Fork::new(base_stream))
    }
}
