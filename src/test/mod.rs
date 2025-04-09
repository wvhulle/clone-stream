#![allow(unused_imports)]
#![allow(dead_code)]

mod clone_stream;
mod metrics;
mod regression;
mod set_log_level;
mod test_setup;
mod time_range;
mod wakers_context;

use core::panic;
use std::{
    fmt::Debug,
    task::{Context, Poll},
    time::Duration,
};

pub use clone_stream::{StreamNextPollError, StreamWithWakers};
use futures::{FutureExt, Stream, StreamExt, task::noop_waker};
use log::{info, trace};
pub use metrics::{average_time_to_resume_and_receive, average_warmup, spacing_wide_enough};
pub use regression::{find_average_min, floats_from_to, ints_from_to};
pub use set_log_level::log_init;
pub use test_setup::ForkAsyncMockSetup;
pub use time_range::TimeRange;
pub use wakers_context::MockWaker;

use crate::ForkStream;
