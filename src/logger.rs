use std::fs;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logger(debug: bool) {
    let log_dir = PathBuf::from(".deep/logs");
    let _ = fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::never(".deep/logs", "agent.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = if debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(file_layer)
        .init();

    // Box::leak to keep the guard alive for the duration of the program
    Box::leak(Box::new(_guard));
}
