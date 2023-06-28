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
    database::log_location(loc.clone()).await.unwrap();
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
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(fname)
        .unwrap();
    writeln!(file, "{}", line).unwrap();
}

/// Print a line of log output and log it to stemlog.txt. Can only be called
/// after AppState has been initialized.
pub fn print_and_log(line: &str) {
    println!("{}", line);
    let docdir = get_documents_dir();
    log_with_dir(line, &docdir);
}

// TODO: get log crate working
pub fn timestamp() -> String {
    let now = time::OffsetDateTime::now_utc()
        .to_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap());
    now.format(&time::format_description::well_known::Rfc2822)
        .unwrap()
}

pub fn debug(msg: &str) {
    print_and_log(&format!("DEBUG {} {}", timestamp(), msg))
}
