use std::time::Duration;

use tokio::time::{Instant, sleep_until};

pub fn log() {
    let _ = env_logger::builder()
        .format_file(true)
        .format_target(false)
        .format_level(false)
        .format_timestamp_millis()
        .format_line_number(true)
        .filter_level(log::LevelFilter::Trace)
        .format_module_path(false)
        .try_init();
}

pub fn until(start: Instant, n: usize) -> impl Future<Output = ()> {
    sleep_until(start + Duration::from_millis(10) * n as u32)
}
