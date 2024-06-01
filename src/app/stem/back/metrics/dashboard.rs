//! Updater for dashboard statistics.

use std::cmp::Ordering;

use common::{
    dashboard_metrics::DashboardMetrics,
    state::{MapState, PersistedRoute},
    Location,
};
use itertools::Itertools;

use crate::{
    app_state::{get_front_state, set_derived_state},
    core::new_data_is_visible,
    database,
    geojson::{BOUND_EXPANSION, DECIMATION_THRESHOLD},
    runtime::get_runtime,
};

use super::distance::distance_between_locations;

/// Update the dashboard, but only under certian conditions.
///
/// Updates if
/// (on dashboard route)
///     && (no new location data || new location data is visible)
///
/// Should only be called if the app is foregrounded or will be foregrounded
/// soon.
pub async fn update_dashboard(new_loc: Option<Location>) {
    let map_state =
        get_front_state(|front| front.as_ref().unwrap().map.clone());
    if !should_update(&new_loc, &map_state) {
        return;
    };
    // fetch the records, sorted by timestamp
    let records = database::FilteredQuery::new()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(BOUND_EXPANSION))
        .limit(DECIMATION_THRESHOLD)
        .fetch_decimated()
        .await;
    update_stats(&records);
    // update_plot(&records, &map_state);
}

/// Update the scalar statistics.
fn update_stats(records: &[Location]) {
    let count = records.len();
    let total_distance: f64 = records
        .iter()
        .tuple_windows::<(_, _)>()
        .map(|(a, b)| distance_between_locations(a, b))
        .sum();
    let start_time = records.first().map(|l| l.timestamp);
    let end_time = records.last().map(|l| l.timestamp);
    let duration = start_time.zip(end_time).map(|(start, end)| end - start);

    let avg_speed = duration.map(|d| total_distance / d.as_seconds_f64());

    let only_speeds = records.iter().filter_map(|l| l.speed);
    let min_speed = only_speeds
        .clone()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let max_speed =
        only_speeds.max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    set_derived_state(|s| {
        s.dashboard_metrics = DashboardMetrics {
            count: count as u64,
            total_distance: Some(total_distance),
            start_time,
            end_time,
            duration,
            min_speed,
            max_speed,
            avg_speed,
            // ..s.dashboard_metrics
        };
    });
}

/// Update the plotly plot showing the current datastream.
// fn update_plot(records: &[Location], map_state: &MapState) {
// let colored_datastream = map_state.style.colored_datastream;
// let offset = map_state.time_range.start.offset();
// let records = records.iter().collect::<Vec<_>>();
// let cmap_params = colored_datastream.get_cmap_params(&records, &offset);
// }

/// Calculates whether to update, as described in the `update_dashboard`
/// docstring.
fn should_update(new_loc: &Option<Location>, map_state: &MapState) -> bool {
    let route = get_front_state(|front| front.as_ref().map(|f| f.route));
    if route != Some(PersistedRoute::Metrics) {
        // not viewing the dashboard -> don't update
        return false;
    }
    if let Some(loc) = new_loc {
        if !new_data_is_visible(loc, map_state, true) {
            return false;
        }
    }
    true
}

/// Update the dashboard statistics on navigation to the page.
pub fn update_dashboard_on_navigate(
    prev_route: Option<PersistedRoute>,
    new_route: PersistedRoute,
) {
    if new_route == PersistedRoute::Metrics
        && prev_route != Some(PersistedRoute::Metrics)
    {
        get_runtime().spawn(async move {
            update_dashboard(None).await;
        });
    }
}
