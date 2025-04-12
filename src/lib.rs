mod bridge;
mod stream;
pub mod test_helpers;

use bridge::Bridge;
use futures::Stream;
pub use stream::CloneStream;
pub use test_helpers::{
    LOAD_TOKIO_TASK, TestSetup, TimeRange, average_time_to_resume_and_receive, average_warmup,
    enable_debug_log, find_average_min, floats_from_to, fork_warmup, ints_from_to,
    min_spacing_seq_polled_forks, spacing_wide_enough, time_per_fork_to_receive_cold,
    tokio_task_load_time,
};
/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream
/// that yields items of the same original item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Bridge::new(self))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
