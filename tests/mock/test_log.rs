use std::io::Write;

use log::LevelFilter;

pub fn log_init() {
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
        .filter_level(LevelFilter::Debug)
        .filter(Some("clone_stream"), LevelFilter::Trace)
        .try_init();
}
