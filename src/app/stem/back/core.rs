//! High-level app logic that spans multiple modules.
//!
//! E.g. when we get new location data, we want to
//!     1) store it in the database
//!     2) send an update to the UI
//!     3) do any additional calculations for system visibility or user
//!             analysis.

use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::app_state::AppState;
use crate::database::{self, OSLocationData};
use crate::geojson::update_geojson;
use crate::paths::get_documents_dir;
use crate::ws_session;

pub async fn log_location(loc: OSLocationData) {
    // Log the location in our database
    if let Err(e) = database::log_location(loc.clone()).await {
        error("Failed to log location.", e);
    }
    // We first want to get the address, NOT in the "if let" scrutinee, since
    // the lock will be held for the whole if-block, and we won't be able to
    // await
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    // If the UI is active, we'll send it the new location to display
    if let Some(addr) = maybe_addr {
        addr.do_send(ws_session::SendState);
        tokio::spawn(update_geojson(Some(loc.into()), false));
    }
}

/// Log a line of debug information to stemlog.txt, inside the documents
/// directory provided. Used during startup before AppState is initialized.
pub fn log_with_dir(line: &str, docdir: &std::path::Path) {
    let fname = docdir.join("stemlog.txt");
    match OpenOptions::new().create(true).append(true).open(fname) {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", line) {
                println!("Failed to write error to file. Err: {e}");
            }
        }
        // not great if we can't log our errors, but we settle for printing
        Err(e) => println!("Failed open log file. Err: {e}"),
    }
}

/// Print a line of log output and log it to stemlog.txt. Can only be called
/// after AppState has been initialized.
pub fn print_and_log(line: &str) {
    println!("{}", line);
    let docdir = get_documents_dir();
    log_with_dir(line, &docdir);
}

// TODO: get log crate working
/// Timestamp for debug and error logging
pub fn timestamp() -> String {
    let offset =
        time::UtcOffset::from_hms(-7, 0, 0).unwrap_or(time::UtcOffset::UTC);
    let now = time::OffsetDateTime::now_utc().to_offset(offset);
    match now.format(&time::format_description::well_known::Rfc2822) {
        Ok(formatted) => formatted,
        // don't call error() here because it might recurse!
        Err(e) => format!("[Unformattable timestamp, Err: {e}]"),
    }
}

/// Debug messages are only outputted to the log if the feature
/// extra_debug_logging is turned on, which only happens during dev.
#[allow(unused_variables)]
pub fn debug(msg: &str) {
    #[cfg(feature = "extra_debug_logging")]
    print_and_log(&format!("DEBUG {} {msg}", timestamp()))
}

/// Print and log an error message, and pass it through.
/// The returned error can be ignored or unwrapped if it is significant to the
/// function of the program, and should cause a visible crash.
pub fn error(msg: &str, err: impl Into<anyhow::Error>) -> anyhow::Error {
    let err = err.into();
    print_and_log(&format!("ERROR {} {msg} Err: {err}", timestamp()));
    err
}
