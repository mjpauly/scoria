//! High-level app logic that spans multiple modules.
//!
//! E.g. when we get new location data, we want to
//!     1) store it in the database
//!     2) send an update to the UI
//!     3) do any additional calculations for system visibility or user
//!             analysis.

use common::state::{MapState, PendingEvents};
use common::ToFront;
use tracing::error;

use crate::app_state::AppState;
use crate::database::{self, OSLocationData};
use crate::map::automap::update_automap;
use crate::map::geojson::{update_geojson, BOUND_EXPANSION};
use crate::metrics::dashboard::update_dashboard;
use crate::ws_session::send_message_to_front;
use crate::{logs, ws_session};

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

/// When the frontend connects, update state we want to display with new data
/// that may have come in when backgrounded.
///
/// Note that if this is the first time opening the app, we might not have the
/// front state yet!
pub fn update_on_foregrounding() {
    tokio::spawn(update_geojson(None, true));
    tokio::spawn(update_automap());
    tokio::spawn(async {
        if let Err(e) = logs::update_last_logged_error().await {
            tracing::error!("IO failure when updating last logged error: {e}");
        };
    });
    tokio::spawn(database::pins::update_derived_pins());
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

/// Handle an incoming `scoria://` url scheme, which may come in before we've
/// foregrounded.
///
/// e.g.:
/// ```text
/// scoria://place?name=Ferry+Building&lng=-122.39339582391952&lat=\
/// 37.79552680112931&icon=%E2%9B%B4%EF%B8%8F
/// ```
pub fn handle_url_scheme(url: String) {
    let binding = AppState::global();
    let mut guard = binding.pending_events.lock().unwrap();
    if guard.is_none() {
        *guard = Some(PendingEvents::default());
    }
    guard.as_mut().map(|events| events.opened_url = Some(url));
    // if frontend is connected, directly send the events
    if AppState::global().ws_addr.lock().unwrap().is_some() {
        if let Some(events) = guard.take() {
            send_message_to_front(ToFront::PendingEvents(events));
        }
    }
}
