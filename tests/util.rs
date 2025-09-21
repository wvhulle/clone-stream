#![allow(dead_code)]
use std::{future::Future, time::Duration};

use tokio::time::{Instant, sleep_until};

pub fn until(start: Instant, n: usize) -> impl Future<Output = ()> {
    sleep_until(start + Duration::from_millis(10) * n as u32)
}
