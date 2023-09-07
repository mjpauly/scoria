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

use tracing::Subscriber;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::paths::{get_logs_dir_helper, Paths};

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
