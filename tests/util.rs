use std::time::Duration;

use tokio::time::{Instant, sleep_until};

pub fn log() {
    let _ = env_logger::builder()
        .format(|buf, record| {
            use std::io::Write;
            let file = record.file().unwrap_or("unknown");
            let relative_file = file
                .strip_prefix(&*std::env::current_dir().unwrap().to_string_lossy())
                .unwrap_or(file)
                .trim_start_matches('/');

            let (color_start, color_end) = match record.level() {
                log::Level::Error => ("\x1b[91m", "\x1b[0m"), // Red
                log::Level::Warn => ("\x1b[93m", "\x1b[0m"),  // Yellow
                log::Level::Info => ("\x1b[34m", "\x1b[0m"),  // Dark blue
                log::Level::Debug => ("\x1b[96m", "\x1b[0m"), // Cyan (lighter blue)
                log::Level::Trace => ("\x1b[90m", "\x1b[0m"), // Gray
            };

            writeln!(
                buf,
                "{}{}:{} {}{}",
                color_start,
                relative_file,
                record.line().unwrap_or(0),
                record.args(),
                color_end
            )
        })
        .filter_level(log::LevelFilter::Trace)
        .try_init();
}

#[allow(dead_code)]
pub fn until(start: Instant, n: usize) -> impl Future<Output = ()> {
    sleep_until(start + Duration::from_millis(10) * n as u32)
}
