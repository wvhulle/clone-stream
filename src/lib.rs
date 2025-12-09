//! # Clone streams with `clone-stream`
//!
//! Lazy single-threaded stream cloning: items are only cloned when a consumer
//! actually polls for them.
//!
//! The [`CloneStream`] struct implements [`Clone`] + [`Stream`], allowing you
//! to create multiple concurrent consumers. The [`ForkStream`] trait provides
//! the entry point via the [`fork()`](ForkStream::fork) method.
//!
//! # How It Works
//!
//! Unlike broadcast channels that eagerly clone every item for every
//! subscriber, this crate clones **on-demand**. Items are delivered only to
//! clones actively polling when items arrive (poll-time semantics).
//!
//! # Important: Single-Threaded Runtime Required
//!
//! This crate requires single-threaded async runtimes (`current_thread` flavor
//! or `#[tokio::test]`). The lazy semantics depend on cooperative scheduling;
//! multi-threaded runtimes cause race conditions.
pub mod clean_log;
mod clone;
mod error;
mod fork;
mod registry;
pub mod ring_queue;
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
    fn from(base_stream: BaseStream) -> Self {
        Self::from(Fork::new(base_stream))
    }
}
