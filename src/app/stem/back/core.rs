//! High-level app logic that spans multiple modules.
//!
//! E.g. when we get new location data, we want to
//!     1) store it in the database
//!     2) send an update to the UI
//!     3) do any additional calculations for system visibility or user
//!             analysis.

use tracing::error;

use crate::app_state::AppState;
use crate::database::{self, OSLocationData};
use crate::geojson::update_geojson;
use crate::map::automap::update_automap;
use crate::ws_session;

pub async fn log_location(loc: OSLocationData) {
    // Log the location in our database
    if let Err(e) = database::log_location(loc.clone()).await {
        error!("Failed to log location: {e}.");
    }
    // We first want to get the address, NOT in the "if let" scrutinee, since
    // the lock will be held for the whole if-block, and we won't be able to
    // await
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    // If the UI is active, we'll send it the new location to display
    if let Some(addr) = maybe_addr {
        addr.do_send(ws_session::SendState);
        tokio::spawn(update_geojson(Some(loc.into()), false));
        tokio::spawn(update_automap());
    }
}
