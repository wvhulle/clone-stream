use std::io::Write;

use log::LevelFilter;

pub fn enable_debug_log() {
    let _ = env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {}:{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.6f"),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Trace)
        .filter(Some("forked_stream"), LevelFilter::Trace)
        .try_init();
}
