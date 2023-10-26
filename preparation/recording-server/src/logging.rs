use std::path::Path;

use miette::Result;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::LevelFilter,
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

/// Initialize the console and file logging.
///
/// If `log_file_directory_path` is `Some`, the logs will be written to the specified directory
/// into a daily-rolling log file.
///
/// **IMPORTANT: Retain the returned
/// [`WorkerGuard`](../tracing_appender/non_blocking/struct.WorkerGuard.html)
/// in scope, otherwise flushing to file will stop.**
pub fn initialize_tracing<P>(
    console_level: Level,
    log_file_level: Level,
    log_file_directory_path: P,
) -> Result<WorkerGuard>
where
    P: AsRef<Path>,
{
    let console_layer = {
        let console_tracing_format = tracing_subscriber::fmt::format()
            .with_ansi(true)
            .with_target(true)
            .with_level(true);

        let console_layer = tracing_subscriber::fmt::layer()
            .log_internal_errors(true)
            .event_format(console_tracing_format);

        let console_level_filter_layer = LevelFilter::from(console_level);

        console_layer.with_filter(console_level_filter_layer)
    };

    let (file_layer, file_guard) = {
        let file_tracing_format = tracing_subscriber::fmt::format()
            .with_ansi(false)
            .with_target(true)
            .with_level(true);

        let (appender, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
            log_file_directory_path,
            "recording-server.log",
        ));

        let file_subscriber = tracing_subscriber::fmt::layer()
            .with_writer(appender)
            .log_internal_errors(true)
            .event_format(file_tracing_format);

        let file_subscriber_filter_layer = LevelFilter::from(log_file_level);

        (
            file_subscriber.with_filter(file_subscriber_filter_layer),
            guard,
        )
    };

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();


    Ok(file_guard)
}
