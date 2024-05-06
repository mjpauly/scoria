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

use std::cmp::Ordering;

use tracing::Subscriber;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::{
    app_state::AppState,
    paths::{get_logs_dir, get_logs_dir_helper, Paths},
};

/// Default subscriber for when the app is running
pub fn get_subscriber(paths: &Paths) -> impl Subscriber + Send + Sync {
    let logdir = get_logs_dir_helper(paths);
    // blocking log to file: directory/log.YYYY-MM-DD-HH
    let log_file = tracing_appender::rolling::hourly(logdir, "log");
    let log = fmt::Layer::new().with_writer(log_file).with_ansi(false);
    // TODO: when deletion of old log files becomes an option with the rolling
    // log, update it to delete old log files (keep, say a week)
    // https://github.com/tokio-rs/tracing/issues/2685

    // set level directive with RUST_LOG, or default to just pass errors
    let env_filter = EnvFilter::from_default_env();

    // also send logs to stderr
    let stderr = fmt::Layer::new().with_writer(std::io::stderr).pretty();
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

// pattern to match in the log files
const ERROR_PATTERN: &str = "Z ERROR ";

/// Update the last error that was logged as tracked in the app state so that
/// it's visible to the frontend. If the new error is significantly newer, then
/// we reset the `reviewed` part. The tracked error is only the contents of the
/// most recent log file with errors detected in it. Since the log files rotate
/// hourly, errors that occur after an hour result in resetting the `reviewed`
/// flag to false.
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
    for file in files {
        let contents = std::fs::read_to_string(file.0)?;
        if contents.contains(ERROR_PATTERN) {
            // save that hour's log contents for review and crash reporting.
            // last_error = Some(contents);
            let app_state = AppState::global();
            let last_logged_error = &mut app_state
                .persistent
                .lock()
                .unwrap()
                .back
                .last_logged_error;
            if let Some(prev) = last_logged_error {
                // previous error exists
                if !is_newer(&contents, &prev.0) {
                    // new log contents not significantly newer than previous ->
                    // just update the string, not whether the error was
                    // reviewed
                    prev.0 = contents;
                    return Ok(());
                }
            }
            // this error is newer or it's the first one -> replace previous
            // error in app state and set `reviewec` to false
            *last_logged_error = Some((contents, false));
            return Ok(());
        }
    }
    Ok(())
}

// number of characters in the time representation in the log file
const LOG_TIME_LEN: usize = 27;

/// Compares the first few characters of two log file strings to see if the left
/// is newer than the right or not. This definition means that a new error is
/// not considered newer until the next rolling log file is started.
/// Nonetheless, we still update the string, we just don't reset the `reviewed`
/// status of the error.
fn is_newer(a: &str, b: &str) -> bool {
    let a = &a[..LOG_TIME_LEN];
    let b = &b[..LOG_TIME_LEN];
    a.cmp(b) == Ordering::Greater
}
