//! Updater for dashboard statistics.

use std::cmp::Ordering;

use common::{
    dashboard_metrics::DashboardMetrics,
    map_style::ColoredDataStream,
    pin::Pin,
    plot_data::TimeSeriesPlot,
    state::{MapState, PersistedRoute},
    timeline::{Dwell, Movement, Period, Timeline},
    LngLat, Location, TimeRange,
};
use itertools::Itertools;
use nav_types::WGS84;
use tracing::{instrument, Level};

use crate::{
    app_state::{get_derived_state, get_front_state, set_derived_state},
    core::new_data_is_visible,
    database,
    geojson::{BOUND_EXPANSION, DECIMATION_THRESHOLD},
    runtime::get_runtime,
};

use super::{
    color::get_colored_data_vals,
    distance::distance_between_locations,
    dwells::{
        dwell_score, long_dwell_threshold, segment_on_visibility,
        weighted_lnglat_mean_and_stddev_without_outliers,
    },
};

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

/// Update the dashboard, but only under certian conditions.
///
/// Updates if
/// (on dashboard route)
///     && (no new location data || new location data is visible)
///
/// Should only be called if the app is foregrounded or will be foregrounded
/// soon.
pub async fn update_dashboard(new_loc: Option<Location>) {
    let Some(map_state) = get_front_state(|front| front.map.clone()) else {
        return;
    };
    if !should_update(&new_loc, &map_state) {
        return;
    };
    // fetch the records, sorted by timestamp
    let records = database::FilteredQuery::new()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(BOUND_EXPANSION))
        .get_adjacent(true)
        .limit(DECIMATION_THRESHOLD)
        .fetch_decimated()
        .await;

    // use info about whether a point is inside the visible view bounds to
    // segment point into continuous pieces
    let records_and_inbounds = records
        .iter()
        .map(|loc| (loc, map_state.view_pos.bounds.contains(&loc.lnglat())))
        .collect::<Vec<_>>();
    let segments = segment_on_visibility(&records_and_inbounds);
    update_stats(&segments);
    // update_plot(&records, &map_state);
}

/// Calculates whether to update, as described in the `update_dashboard`
/// docstring.
fn should_update(new_loc: &Option<Location>, map_state: &MapState) -> bool {
    let route = get_front_state(|front| front.route);
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

/// Update the dashboard stats for all segments visible in the map view.
fn update_stats(segments: &[(bool, Vec<&Location>)]) {
    let mut total_stats = DashboardMetrics::default();
    let mut timeline = Timeline::new();
    let pins = get_derived_state(|s| s.pins.clone());
    for (visible, seg) in segments.iter() {
        if *visible {
            let is_dwell = long_dwell_threshold(dwell_score(seg).iter());
            let records_and_isdwell = seg
                .iter()
                .copied()
                .zip(is_dwell.into_iter())
                .collect::<Vec<_>>();
            total_stats = total_stats
                .merge_other(&get_segment_stats(&records_and_isdwell));
            if !timeline.is_empty() {
                timeline.push(Period::Unknown);
            }
            timeline.extend_from_slice(&resolve_timeline_activities(
                &records_and_isdwell,
                &pins,
            ));
        }
    }
    set_derived_state(|s| {
        s.dashboard_metrics = total_stats;
        s.timeline = timeline;
    });
}

/// Items are locations and whether the location is part of a dwell, where
/// distances are not calculated due to excess dithering.
fn get_segment_stats(records: &[(&Location, bool)]) -> DashboardMetrics {
    let count = records.len();
    let total_distance: f64 = records
        .iter()
        .filter(|(_, dwell)| !dwell) // don't consider points in dwells
        .tuple_windows::<(_, _)>()
        .map(|((a, _), (b, _))| distance_between_locations(a, b))
        .sum();
    let mut dwell_time = time::Duration::ZERO;
    let mut movement_time = time::Duration::ZERO;
    for ((a, adwell), (b, bdwell)) in records.iter().tuple_windows::<(_, _)>() {
        let dur = b.timestamp - a.timestamp;
        if *adwell || *bdwell {
            // count transitions between dwells and non-dwells as part of the
            // dwells, since there's often a long delay between the end of the
            // dwell and the beginning of the next movement
            dwell_time += dur;
        } else {
            movement_time += dur;
        }
    }

    let start_time = records.first().map(|(l, _)| l.timestamp);
    let end_time = records.last().map(|(l, _)| l.timestamp);

    let avg_speed = (movement_time > time::Duration::ZERO)
        .then(|| total_distance / movement_time.as_seconds_f64());

    let only_speeds =
        records
            .iter()
            .filter_map(|(l, isdwell)| if !isdwell { l.speed } else { None });
    // TODO: throw out outliers?
    let min_speed = only_speeds
        .clone()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let max_speed =
        only_speeds.max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    DashboardMetrics {
        count: count as u64,
        total_distance,
        start_time,
        end_time,
        dwell_time,
        movement_time,
        min_speed,
        max_speed,
        avg_speed,
    }
}

#[allow(unused)]
fn resolve_timeline_activities(
    records: &[(&Location, bool)],
    pins: &[Pin],
) -> Timeline {
    let mut timeline = Timeline::new();
    let chunker = records
        .iter()
        .tuple_windows::<(_, _)>()
        .chunk_by(|((_, da), (_, db))| *da || *db);
    for (isdwell, chunk) in chunker.into_iter() {
        let chunk = chunk.map(|((a, _), (b, _))| (a, b)).collect::<Vec<_>>();
        let first_span = chunk.first().unwrap(); // should exist
        let start = first_span.0.timestamp;
        let end = chunk
            .last()
            .map(|(_, b)| b.timestamp)
            .unwrap_or(first_span.1.timestamp);
        if isdwell {
            let lnglats_and_weights = chunk.iter().map(|(a, b)| {
                (a.lnglat(), (b.timestamp - a.timestamp).as_seconds_f64())
            });
            let (lnglat, _stddev) =
                weighted_lnglat_mean_and_stddev_without_outliers(
                    lnglats_and_weights,
                    None,
                );
            timeline.push(Period::Dwell(Dwell {
                time: TimeRange { start, end },
                lnglat,
            }));
        } else {
            let distance: f64 = chunk
                .iter()
                .map(|(a, b)| distance_between_locations(a, b))
                .sum();
            timeline.push(Period::Movement(Movement {
                time: TimeRange { start, end },
                distance,
            }));
        }
    }
    timeline
}

/// Given the mean and stddev of the dwell location, find the pin that
/// corresponds to the location, if there is a good candidate.
#[allow(unused)]
fn find_nearest_pin(
    location: &LngLat,
    stddev: f64,
    pins: &[Pin],
) -> Option<Pin> {
    let center =
        WGS84::from_degrees_and_meters(location.lat, location.lng, 0.0);
    /*
    let closest = pins.iter().min_by_key(|p| {
        // p_loc = WGS84::from_degrees_and_meters(p.
    });
    */

    todo!()
}

// #[instrument(skip_all, level = Level::TRACE)]
// fn update_timeline(records: &[Location]) {
// let recs = records.iter().map(|l| (l, true)).collect::<Vec<_>>();
// let is_dwell = long_dwell_detect(&recs);
// let zipped = records.iter().zip(is_dwell.into_iter()).collect::<Vec<_>>();
// }

#[instrument(skip_all, level = Level::TRACE)]
fn update_plot(records: &[Location], map_state: &MapState) {
    let colored_datastream = &map_state.style.colored_datastream;
    if !colored_datastream.is_some() {
        return;
    }
    let recs = records.iter().map(|l| (l, true)).collect::<Vec<_>>();
    let offset = map_state.time_range.start.offset();
    let cmap_vals = get_colored_data_vals(colored_datastream, &recs, &offset);

    update_timeseries_plot_data(records.iter(), &cmap_vals, colored_datastream);
}

/// Update timeseries plot data with the current data stream coloring.
///
/// Only points with a valid colorval are included.
#[instrument(skip_all, level = Level::TRACE)]
pub fn update_timeseries_plot_data<'a>(
    records: impl Iterator<Item = &'a Location>,
    cmap_vals: &[Option<f64>],
    colored_datastream: &ColoredDataStream,
) {
    let (t, y): (Vec<_>, Vec<_>) = records
        .zip(cmap_vals.iter())
        .filter_map(|(l, cval)| cval.map(|v| (l.timestamp, v)))
        .unzip();
    let ylabel = get_front_state(|front| front.unit_pref)
        .map(|unit_pref| colored_datastream.name_with_unit(&unit_pref))
        .unwrap_or_else(|| colored_datastream.to_string());
    let timeseries_plot = TimeSeriesPlot { t, y, ylabel };
    set_derived_state(|state| state.colored_timeseries_plot = timeseries_plot);
}
