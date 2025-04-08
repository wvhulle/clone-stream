#![allow(unused_imports)]
#![allow(dead_code)]

mod clone_stream;

mod set_log_level;
mod time_range;
mod wakers_context;

use core::panic;
use std::{
    fmt::Debug,
    task::{Context, Poll},
    time::Duration,
};

pub use clone_stream::{ForkAsyncMockSetup, StreamWithWakers};
use forked_stream::ForkStream;
use futures::{FutureExt, Stream, StreamExt, task::noop_waker};
use log::{info, trace};
pub use set_log_level::log_init;
pub use time_range::TimeRange;
pub use wakers_context::MockWaker;
