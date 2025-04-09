mod bridge;
mod estimates;
mod stream;
mod test;

use bridge::ForkBridge;
pub use estimates::{
    TOKIO_TASK_STARTUP, fork_warmup, resume, resume_forks, spacing_required, wake_up_time, warmup,
};
use futures::Stream;
pub use stream::{CloneStream, Fork};
pub use test::{
    ForkAsyncMockSetup, TimeRange, average_time_to_resume_and_receive, average_warmup,
    find_average_min, floats_from_to, ints_from_to, spacing_wide_enough,
};
/// A trait that turns a `Stream` with cloneable `Item`s into a cloneable stream with the same item type.
pub trait ForkStream: Stream<Item: Clone> + Sized {
    /// Forks the stream into a new stream that can be cloned.
    /// The `max_buffered` parameter controls how many items can be buffered for each fork.
    fn fork(self) -> CloneStream<Self> {
        CloneStream::from(Fork::new(ForkBridge::new(self)))
    }
}

impl<BaseStream> ForkStream for BaseStream where BaseStream: Stream<Item: Clone> {}
