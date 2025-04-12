use log::LevelFilter;

pub fn enable_debug_log() {
    let _ = env_logger::Builder::new()
        .is_test(true)
        .format_timestamp(Some(env_logger::TimestampPrecision::Micros))
        .filter_level(LevelFilter::Trace)
        .format_module_path(false)
        .format_target(false)
        .format_file(true)
        .format_line_number(true)
        .filter(Some("forked_stream"), LevelFilter::Trace)
        .try_init();
}
