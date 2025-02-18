use anyhow::{Context, Result};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::Layer as FmtLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// Setup logger configuration for the application
///
/// If LOG_INSIDE_FILE=true:
/// - Console output for all log levels
/// - A combined log file for all levels
/// - A separate file for warnings only
/// - A separate file for errors only
///
/// If LOG_INSIDE_FILE=false (default):
/// - Only console output for all log levels
///
/// All logs are rotated daily when file logging is enabled
pub fn setup_logger() -> Result<()> {
    let log_inside_file: bool = std::env::var("LOG_INSIDE_FILE")
        .unwrap_or("false".to_string())
        .parse()
        .unwrap_or(false);

    // Set default log level to INFO if RUST_LOG is not set
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Layer for console output
    let console_layer = FmtLayer::new()
        .with_line_number(false)
        .with_target(false)
        .with_thread_ids(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    if log_inside_file {
        // Configure the combined logs appender
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("combined")
            .build(".logs/combined.log")
            .context("Failed to create combined logs appender")?;

        // // New appender for info logs
        // let info_appender = RollingFileAppender::builder()
        //     .rotation(Rotation::DAILY)
        //     .filename_prefix("info")
        //     .build(".logs/info.log")
        //     .unwrap();

        // Configure the warnings-only appender
        let warn_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("warn")
            .build(".logs/warn.log")
            .context("Failed to create warnings-only appender")?;

        // Configure the errors-only appender
        let error_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("error")
            .build(".logs/error.log")
            .context("Failed to create errors-only appender")?;

        // Layer for combined logs
        let file_layer = FmtLayer::new()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_thread_ids(false);

        // Layer for info logs
        // let info_layer = FmtLayer::new()
        //     .with_writer(info_appender)
        //     .with_ansi(false)
        //     .with_thread_ids(false)
        //     .with_filter(EnvFilter::new("info"));

        // Layer for warning logs
        let warn_layer = FmtLayer::new()
            .with_writer(warn_appender)
            .with_ansi(false)
            .with_thread_ids(false)
            .with_filter(EnvFilter::new("warn"));

        // Layer for error logs
        let error_layer = FmtLayer::new()
            .with_writer(error_appender)
            .with_ansi(false)
            .with_thread_ids(false)
            .with_filter(EnvFilter::new("error"));

        // Initialize with all layers
        registry
            .with(file_layer)
            // .with(info_layer)
            .with(warn_layer)
            .with(error_layer)
            .init();
    } else {
        // Initialize with console layer only
        registry.init();
    }

    Ok(())
}
