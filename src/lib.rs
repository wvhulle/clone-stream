mod bridge;
mod estimates;
mod fork;
mod stream;
mod test;

use bridge::Bridge;
pub use estimates::{
    TOKIO_TASK_STARTUP, space_required_consecutive_fork_polls, time_fork_needs_to_wake_and_receive,
    time_start_tokio_task, time_to_receive_after_send, time_to_receive_on_fork_after_input,
};
pub use fork::Fork;
use futures::Stream;
pub use stream::CloneStream;
pub use test::{
    TestSetup, TimeRange, average_time_to_resume_and_receive, average_warmup, enable_debug_log,
    find_average_min, floats_from_to, ints_from_to, spacing_wide_enough,
};
/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    /// The `max_buffered` parameter controls how many items can be buffered for each fork.
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Fork::from(Bridge::new(self)))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
