//! High-level app logic that spans multiple modules.
//!
//! E.g. when we get new location data, we want to
//!     1) store it in the database
//!     2) send an update to the UI
//!     3) do any additional calculations for system visibility or user
//!             analysis.

use common::state::MapState;
use tracing::error;

use crate::app_state::AppState;
use crate::database::{self, OSLocationData};
use crate::geojson::{update_geojson, BOUND_EXPANSION};
use crate::map::automap::update_automap;
use crate::metrics::dashboard::update_dashboard;
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
        let new_loc = Some(loc.into());
        tokio::spawn(update_geojson(new_loc.clone(), false));
        tokio::spawn(update_automap());
        tokio::spawn(update_dashboard(new_loc));
    }
}

/// Update derived state at startup, since it is not peristed between launches.
pub async fn update_derived_state() {
    // pins are not persisted in persistent_state.json, but are pulled from the
    // database when needed into DerivedState
    tokio::spawn(database::pins::update_derived_pins());
    tokio::spawn(update_dashboard(None));
}

/// Determine if a new location data point would be visible on the map.
///
/// Compares the data to the current time range bounds, map view bounds (if view
/// bounded), and active filters.
pub fn new_data_is_visible(
    loc: &common::Location,
    map_state: &MapState,
    view_bounded: bool,
) -> bool {
    let in_time_range = map_state.time_range.contains(&loc.timestamp);
    let in_bounds = !view_bounded
        || map_state
            .view_pos
            .bounds
            .expand(BOUND_EXPANSION)
            .contains(&loc.lnglat());
    let not_filtered_out =
        !map_state.filters.iter().any(|f| f.should_remove(loc));
    in_time_range && in_bounds && not_filtered_out
}
