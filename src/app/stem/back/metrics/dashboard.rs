//! Updater for dashboard statistics.

use std::{cmp::Ordering, collections::BTreeMap};

use common::{
    cmaps::CmapParams,
    dashboard_metrics::DashboardMetrics,
    map_style::ColoredDataStream,
    mounted::MountID,
    pin::Pin,
    plot_data::TimeSeriesPlot,
    state::{MapState, PersistedRoute},
    timeline::{Dwell, Movement, Period, PeriodKind, Timeline},
    LngLat, Location,
};
use itertools::Itertools;
use nav_types::WGS84;
use tracing::{instrument, Level};

use crate::{
    app_state::{
        get_derived_state, get_front_state, set_derived_state, AppState,
    },
    core::new_data_is_visible,
    database,
    map::geojson::{BOUND_EXPANSION, DECIMATION_THRESHOLD},
    metrics::distance::straight_distance,
    tz::{datetime_fn_infallible, location_datetime_fn},
};

use super::{
    distance::distance_between_locations,
    dwells::{
        dwell_score, long_dwell_threshold, segment_on_visibility,
        weighted_lnglat_mean_and_stddev_without_outliers,
    },
};

/// Update the dashboard statistics on navigation to the page unconditionally
/// (since new data may have come in), or when the map state has changed.
pub async fn update_dashboard_on_state_change(
    prev_route: Option<PersistedRoute>,
) {
    let app_state = AppState::global();
    let mut prev_map_data_guard =
        app_state.map_data.prev_map_state_dashboard.lock().await;
    let Some(map_state) = get_front_state(|s| s.map.clone()) else {
        return;
    };
    let map_state_different = prev_map_data_guard
        .as_ref()
        .map(|prev_state| map_state_is_different(prev_state, &map_state))
        .unwrap_or(true); // if no prev map data, assume we need to update
    let navigated = prev_route != Some(PersistedRoute::Metrics);
    if navigated || map_state_different {
        *prev_map_data_guard = Some(map_state);
        update_dashboard(None).await;
    }
}

/// Determine if the map state has changed such that the dashboard should
/// update.
fn map_state_is_different(prev: &MapState, curr: &MapState) -> bool {
    prev.time_range != curr.time_range
        || prev.filters != curr.filters
        || prev.view_pos != curr.view_pos
        || prev.timeline_config != curr.timeline_config
}

/// Update the dashboard with new location data or unconditionally if None.
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
    let records = database::FilteredQuery::builder()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(BOUND_EXPANSION))
        .get_adjacent(true)
        .limit(DECIMATION_THRESHOLD)
        .build()
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
    let mut pins = get_derived_state(|s| s.pins.clone());
    let pin_filters = get_front_state(|s| s.pin_settings.filters.clone());
    if let Some(filters) = pin_filters {
        pins = pins
            .into_iter()
            .filter(|p| p.passes_filters(&filters))
            .collect::<Vec<_>>();
    }
    for (visible, seg) in segments.iter() {
        if *visible {
            let is_dwell = long_dwell_threshold(dwell_score(seg).into_iter());
            let records_and_isdwell = seg
                .iter()
                .copied()
                .zip(is_dwell.into_iter())
                .collect::<Vec<_>>();
            total_stats = total_stats
                .merge_other(&get_segment_stats(&records_and_isdwell));
            let new_timeline_part =
                resolve_timeline_activities(&records_and_isdwell, &pins);
            if !new_timeline_part
                .iter()
                .any(|a| matches!(a.kind, PeriodKind::Dwell(_)))
            {
                // skip any contiguous regions without dwell data, e.g. if the
                // map edge slices through some activity that gets detected as
                // a bunch of short movement-only segments.
                continue;
            }
            if let Some(last) = timeline.last() {
                if let Some(next) = new_timeline_part.first() {
                    // anytime the location track leaves the view bounds we have
                    // a Period::Unknown
                    timeline.push(Period {
                        kind: PeriodKind::Unknown,
                        start: last.end.clone(),
                        end: next.start.clone(),
                    });
                }
            }
            timeline.extend_from_slice(&new_timeline_part);
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

/// Given a contiguous segment of activity marked with whether a dwell occurs,
/// and given a list of the pins to consider, extract a timeline of the
/// activities.
fn resolve_timeline_activities(
    records: &[(&Location, bool)], // bool is whether it's a dwell
    pins: &[Pin],
) -> Timeline {
    let mut timeline = Timeline::new();
    let datetime_f = datetime_fn_infallible();
    let chunker = records
        .iter()
        .tuple_windows::<(_, _)>()
        .chunk_by(|((_, da), _)| *da);
    for (isdwell, chunk) in chunker.into_iter() {
        let chunk = chunk.map(|((a, _), (b, _))| (a, b)).collect::<Vec<_>>();
        let first_pt = chunk.first().unwrap().0; // must exist
        let last_pt = chunk.last().unwrap().1; // must exist
        let start = datetime_f(first_pt);
        let end = datetime_f(last_pt);
        if isdwell {
            let lnglats_and_weights = chunk.iter().map(|(a, b)| {
                (a.lnglat(), (b.timestamp - a.timestamp).as_seconds_f64())
            });
            let (lnglat, deviation) =
                weighted_lnglat_mean_and_stddev_without_outliers(
                    lnglats_and_weights,
                    None,
                );
            let detected_pin = detect_pin(&lnglat, deviation, pins);
            timeline.push(Period {
                kind: PeriodKind::Dwell(Dwell {
                    lnglat,
                    deviation,
                    detected_pin,
                }),
                start,
                end,
            });
        } else {
            let distance: f64 = chunk
                .iter()
                .map(|(a, b)| distance_between_locations(a, b))
                .sum();
            timeline.push(Period {
                kind: PeriodKind::Movement(Movement { distance }),
                start,
                end,
            });
        }
    }
    timeline
}

// The nearest pin must be within 3 standard deviations of the center of the
// dwell cluster.
const PIN_THRESH_Z_SCORE: f64 = 3.0;

/// Given the mean and deviation of the dwell location, find the pin that
/// corresponds to the location, if there is a good candidate.
///
/// Returns the pin and the distance to the pin. A pin is returned only if the
/// closest one is within PIN_THRESH_Z_SCORE standard deviations away from the
/// dwell cluster.
fn detect_pin(
    location: &LngLat,
    deviation: f64,
    pins: &[Pin],
) -> Option<(Pin, f64)> {
    let center =
        WGS84::from_degrees_and_meters(location.lat, location.lng, 0.0);
    pins.iter()
        .map(|p| {
            let pos =
                WGS84::from_degrees_and_meters(p.lnglat.lat, p.lnglat.lng, 0.0);
            (p, straight_distance(&center, &pos))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        .and_then(|(p, dist)| {
            (dist / deviation < PIN_THRESH_Z_SCORE).then_some((p.clone(), dist))
        })
}

/// Type as returned by get_cmap_data
pub type CmapData = Option<(CmapParams, Vec<Option<f64>>)>;

#[instrument(skip_all, level = Level::TRACE)]
pub fn update_timeseries_plot_data(
    records_and_cmap_data: &BTreeMap<MountID, (Vec<Location>, CmapData)>,
    colored_datastream: &ColoredDataStream,
) {
    let zdt_f = location_datetime_fn();
    let result: BTreeMap<MountID, TimeSeriesPlot> = records_and_cmap_data
        .iter()
        .filter_map(|(mount_id, (records, cmap_data))| {
            // only if there's cmap data
            cmap_data.as_ref().map(|(_params, cmap_vals)| {
                let (t, y): (Vec<_>, Vec<_>) = records
                    .iter()
                    .zip(cmap_vals.iter())
                    .filter_map(|(l, cval)| {
                        cval.and_then(|v| zdt_f(l).ok().map(|zdt| (zdt, v)))
                    })
                    .unzip();
                let ylabel = get_front_state(|front| front.unit_pref)
                    .map(|unit_pref| {
                        colored_datastream.name_with_unit(&unit_pref)
                    })
                    .unwrap_or_else(|| colored_datastream.to_string());
                let timeseries_plot = TimeSeriesPlot { t, y, ylabel };
                (*mount_id, timeseries_plot)
            })
        })
        .collect();
    set_derived_state(|state| state.colored_timeseries_plot = result);
}
