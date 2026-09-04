//! # Error logging
//!
//! ordering:
//!     error, warn, info, debug, trace
//! mnemonic:
//!     "Exploring wondrous information, discovering treasures."
//!
//! # Directives
//!
//! The directive for setting the log level has the syntax:
//! ```
//! target[span{field=value}]=level
//! ```
//!
//! During dev, set the log level with
//! ```
//! RUST_LOG=error,stem=trace ibazel run :dev
//! ```
//!
//! If RUST_LOG is not set, the EnvFilter will only pass errors.
//!
//! # Static Max Level
//!
//! Less important messages are statically ignored during compilation according
//! to the feature flags given to `tracing` in the crate index:
//!
//! ```
//! # statically remove tracing instrumentation at levels above error
//! # for release builds
//! features = ["release_max_level_error"],
//! ```
//!
//! Release builds occur when compiling with optimizations (`-c opt`).
//!
//! # Log compatibility
//!
//! tracing-subscriber's default feature `tracing-log` allows
//! SubscriberInitExt::init() to enable `log` crate compatibility.

use std::{
    cmp::Ordering,
    fmt::Display,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use tracing::Subscriber;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use common::state::LoggedError;

use crate::{
    app_state::set_derived_state,
    paths::{get_logs_dir, get_logs_dir_helper, Paths},
};

/// Take a non-essential operation and log the error without altering control flow or showing a popup to the user.
pub trait LogErrorAndContinue {
    fn log_error_and_continue(&self);
}

impl<T, E: Display> LogErrorAndContinue for std::result::Result<T, E> {
    fn log_error_and_continue(&self) {
        if let Err(e) = self {
            // use the alternate display implementation to show error source
            // details for anyhow errors, not just top-level error
            tracing::error!("{e:#}");
        }
    }
}

/// Default subscriber for when the app is running
pub fn get_subscriber(paths: &Paths) -> impl Subscriber + Send + Sync {
    let logdir = get_logs_dir_helper(paths);
    // blocking log to file: directory/log.YYYY-MM-DD-HH
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::HOURLY)
        .filename_prefix("log")
        .max_log_files(10) // keep only a couple hours worth of error logs
        .build(logdir)
        .unwrap();
    let log = fmt::Layer::new()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_line_number(true);

    // set level directive with RUST_LOG, or default to just pass errors
    let env_filter = EnvFilter::from_default_env();

    // also send logs to stderr
    let stderr = fmt::Layer::new()
        .with_writer(std::io::stderr)
        .pretty()
        .with_span_events(fmt::format::FmtSpan::CLOSE);
    // stderr sent to Android Studio's logcat, which doesn't support ANSI colors
    #[cfg(feature = "android_config")]
    let stderr = stderr.with_ansi(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr)
        .with(log)
}

/// Standard, panicking version of the logging initialization. Must be called
/// only once. We use this version for the app because we don't want to proceed
/// without logging successfully initialized.
#[cfg(not(test))]
pub fn init_logging(subscriber: impl Subscriber + Send + Sync) {
    // With the `tracing-log` feature enabled in tracing-subscriber, this
    // will also initialize a `log` compatibility layer. This is othersise the
    // same as calling `tracing::subscriber::set_global_default(subscriber)`.
    subscriber.init();
    // Log panics as error events. (panics come from the `panic` module)
    log_panics::init();

    tracing::info!("Initialized logging");
}

/// Re-entrant logging initializer for unit tests.
#[cfg(test)]
pub fn init_logging(subscriber: impl Subscriber + Send + Sync) {
    // can be called multiple times by the units tests.
    let _ = subscriber.try_init();
    log_panics::init();
    tracing::info!("Initialized logging");
}

pub fn log_frontend_error(s: String) {
    tracing::error!(target: "frontend", "{s}");
}

// pattern to match in the log files
const ERROR_PATTERN: &str = "Z ERROR ";

/// Most bytes of a log file kept for review and problem reports. Keeps the
/// tail, since that's where the newest messages are.
const MAX_LOG_BYTES: u64 = 64 * 1024;

/// Update the last error that was logged, as tracked in the derived state so
/// it's visible to the frontend. The tracked error is the contents of the most
/// recent hourly log file with errors in it. Whether the user reviewed it is
/// tracked separately by file name in `BackState::reviewed_error_log`, so
/// errors in a later hour's file prompt the user again.
pub async fn update_last_logged_error() -> std::io::Result<()> {
    let logdir = get_logs_dir();
    let entries = std::fs::read_dir(logdir)?;
    let mut files = vec![];
    for entry in entries {
        let entry = entry?;
        // file path and modified time
        files.push((entry.path(), entry.metadata()?.modified()?));
    }
    // sort by modified time descending (recently modified first)
    files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    let mut found = None;
    for (path, _) in files {
        let contents = read_tail(&path, MAX_LOG_BYTES)?;
        if contents.contains(ERROR_PATTERN) {
            let log_file = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            found = Some(LoggedError { log_file, contents });
            break;
        }
    }
    set_derived_state(|derived| derived.last_logged_error = found);
    Ok(())
}

/// Read up to the last `max_bytes` of a file, starting at a line boundary if
/// truncated.
fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let truncated = len > max_bytes;
    if truncated {
        file.seek(SeekFrom::Start(len - max_bytes))?;
    }
    let mut bytes = Vec::with_capacity(len.min(max_bytes) as usize);
    file.read_to_end(&mut bytes)?;
    let mut contents = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        let start = contents.find('\n').map(|i| i + 1).unwrap_or(0);
        contents = format!("[log truncated]\n{}", &contents[start..]);
    }
    Ok(contents)
}
