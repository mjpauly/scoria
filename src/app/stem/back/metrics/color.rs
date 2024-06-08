//! Colormap calculations for data.

use common::{
    cmaps::{Cmap, CmapParams},
    float,
    map_style::ColoredDataStream,
    Location,
};
use itertools::Itertools;
use tracing::{instrument, Level};

use crate::metrics::dashboard::update_timeseries_plot_data;

use super::distance::distance_between_locations;

/// Calculate the cmap parameters and the data values that determine the color
/// given a slice of locations.
pub fn get_cmap_data(
    colored_datastream: &ColoredDataStream,
    // bool indicates if point contributes to the cmap, or will be hidden
    records: &[(&Location, bool)],
    offset: &time::UtcOffset,
) -> Option<(CmapParams, Vec<Option<f64>>)> {
    if !colored_datastream.is_some() {
        return None;
    }
    let cmap_vals = get_colored_data_vals(colored_datastream, records, offset);
    let mut params = CmapParams::default();
    if *colored_datastream == ColoredDataStream::Course {
        params.cmax = 360.;
        params.cmap = Cmap::Twilight;
    } else if *colored_datastream == ColoredDataStream::TimeOfDay {
        params.cmax = 24. * 60. * 60.;
        params.cmap = Cmap::TwilightShifted;
    } else if *colored_datastream == ColoredDataStream::ShortDwellDetection
        || *colored_datastream == ColoredDataStream::LongDwellDetection
    {
        params.cmax = 1.0
    } else {
        let only_valid_data = cmap_vals.iter().filter_map(|x| (*x).as_ref());
        params.cmin = float::min(only_valid_data.clone());
        params.cmax = float::max(only_valid_data);
    }
    debug_assert_eq!(records.len(), cmap_vals.len());
    update_timeseries_plot_data(
        records.iter().map(|r| r.0),
        &cmap_vals,
        colored_datastream,
    );
    Some((params, cmap_vals))
}

/// Computes the colored data values for a slice of locations, returning a
/// vector of the same length with floats for
pub fn get_colored_data_vals(
    colored_datastream: &ColoredDataStream,
    records: &[(&Location, bool)],
    offset: &time::UtcOffset,
) -> Vec<Option<f64>> {
    if *colored_datastream == ColoredDataStream::DistanceDelta {
        delta_color_vals(records, |(a, b)| {
            Some(distance_between_locations(a, b))
        })
    } else if *colored_datastream == ColoredDataStream::TimeDelta {
        delta_color_vals(records, |(a, b)| {
            Some((b.timestamp - a.timestamp).as_seconds_f64())
        })
    } else if *colored_datastream == ColoredDataStream::AvgSpeed {
        delta_color_vals(records, |(a, b)| Some(avg_speed(a, b)))
    } else if *colored_datastream == ColoredDataStream::ShortDwellDetection {
        delta_color_vals(records, |(a, b)| {
            Some(dwell_threshold(
                avg_speed(a, b),
                SHORT_DWELL_SPEED_THRESH_MPS,
            ))
        })
    } else if *colored_datastream == ColoredDataStream::LongDwellDetection {
        min_avg_speed_colorvals(records)
    } else {
        debug_assert!(colored_datastream.is_point_coloring());
        records
            .iter()
            .map(|(l, visible)| {
                visible
                    .then(|| colored_datastream.get_stream(l, offset))
                    .flatten()
            })
            .collect()
    }
}

/// Compute pair-wise data color values given a function to evaluate on each
/// pair of locations.
#[instrument(skip_all, level = Level::TRACE)]
fn delta_color_vals(
    records: &[(&Location, bool)],
    data_fn: impl Fn((&Location, &Location)) -> Option<f64>,
) -> Vec<Option<f64>> {
    let mut out: Vec<_> = records
        .iter()
        .tuple_windows::<(_, _)>()
        // only if first data point, or its line, will be visible
        .map(|(a, b)| a.1.then(|| data_fn((a.0, b.0))).flatten())
        .collect();
    if out.len() < records.len() {
        out.push(None); // final point is uncolored
    }
    out
}

fn avg_speed(a: &Location, b: &Location) -> f64 {
    distance_between_locations(a, b)
        / (b.timestamp - a.timestamp).as_seconds_f64()
}

// 0.5 m/s = 1.8 km/h = 1.1 mi/h
const SHORT_DWELL_SPEED_THRESH_MPS: f64 = 0.5;

const DWELL_WIDTH_SECS: u64 = 120;
const LONG_DWELL_SPEED_THRESH_MPS: f64 = 0.20;

/// Threshold a value and set it to 1.0 if under, or 0.0 if over.
fn dwell_threshold(val: f64, thresh: f64) -> f64 {
    if val < thresh {
        1.0
    } else {
        0.0
    }
}

/// Compute the average speeds between each data point and the previous/next
/// point that is at least `DWELL_WIDTH_SECS` seconds away, then take the
/// minimum between the two values.
///
/// Useful for the detection of dwells and movement. Noisy data may produce
/// large average speeds between adjacent points, even when standing still,
/// so average speeds are computed over a larger time window.
///
/// This is done on two timescales to handle cases where steps are retraced.
/// When retracing steps, a dwell can be falsely detected if only one speed
/// timescale is used. So the average speed between both timescales is used for
/// the forwards and backwards cases.
///
///     distance
///     |       X--c--d    <- dwell
///     |      /
///     |     /
///     |    b             <- movement
///     |   /
///     |  /
///     | a
///     +--------------- time
///
/// In this illustration, a, b, X, c, d are all points. X is the point under
/// consideration. The average speeds between X and all other points are
/// calculated. a and b are in the past, and their speeds to X are averaged to
/// get the `start_speed`. b and c straddle the point X, and their speeds to X
/// are averaged to get the `mid_speed`. c and d are used for the `end_speed`.
///
/// Among all three of these speeds, the minimum is calculated, then thresholded
/// to determine if a dwell occurred.
///
///     distance
///     |             d
///     |            /
///     |           /
///     |    b--X--c       <- dwell
///     |   /
///     |  /
///     | a
///     +--------------- time
///
///     distance
///     |             d
///     |            /
///     |     /-X--c-      <- dwell edge case
///     |    b
///     |   /
///     |  /
///     | a
///     +--------------- time
///
#[instrument(skip_all, level = Level::TRACE)]
fn min_avg_speed_colorvals(records: &[(&Location, bool)]) -> Vec<Option<f64>> {
    let mut ai = 0;
    let mut bi = 0;
    let mut ci = 0;
    let mut di = 0;
    let mut out = vec![];
    let dur_gte_small_thresh = |start: &Location, end: &Location| {
        end.timestamp - start.timestamp
            > time::Duration::seconds(DWELL_WIDTH_SECS as i64 / 2)
    };
    let dur_gte_big_thresh = |start: &Location, end: &Location| {
        end.timestamp - start.timestamp
            > time::Duration::seconds(DWELL_WIDTH_SECS as i64)
    };

    for i in 0..records.len() {
        let curr = records[i];
        out.push(
            curr.1
                .then(|| {
                    // advance the start indices if next value valid
                    while ai < i
                        && dur_gte_big_thresh(records[ai + 1].0, curr.0)
                    {
                        ai += 1;
                    }
                    while bi < i
                        && dur_gte_small_thresh(records[bi + 1].0, curr.0)
                    {
                        bi += 1;
                    }
                    // advance the end index if current end index invalid
                    while ci < records.len() - 1
                        && !dur_gte_small_thresh(curr.0, records[ci].0)
                    {
                        ci += 1;
                    }
                    while di < records.len() - 1
                        && !dur_gte_big_thresh(curr.0, records[di].0)
                    {
                        di += 1;
                    }
                    let a_valid = dur_gte_big_thresh(records[ai].0, curr.0);
                    let b_valid = dur_gte_small_thresh(records[bi].0, curr.0);
                    let c_valid = dur_gte_small_thresh(curr.0, records[ci].0);
                    let d_valid = dur_gte_big_thresh(curr.0, records[di].0);

                    // ensure there's no jumps by checking that intermediate
                    // points are all within the view bounds
                    let no_start_jumps = records[ai..i].iter().all(|x| x.1);
                    let no_mid_jumps = records[bi..ci].iter().all(|x| x.1);
                    let no_end_jumps = records[i..di].iter().all(|x| x.1);

                    let a_speed = avg_speed(records[ai].0, curr.0);
                    let b_speed = avg_speed(records[bi].0, curr.0);
                    let c_speed = avg_speed(curr.0, records[ci].0);
                    let d_speed = avg_speed(curr.0, records[di].0);

                    let start_speed = (a_valid && b_valid && no_start_jumps)
                        .then(|| (a_speed + b_speed) / 2.0);
                    let mid_speed = (b_valid && c_valid && no_mid_jumps)
                        .then(|| (b_speed + c_speed) / 2.0);
                    let end_speed = (c_valid && d_valid && no_end_jumps)
                        .then(|| (c_speed + d_speed) / 2.0);

                    // Some(c_speed)

                    // min_option(start_speed, mid_speed, end_speed)

                    min_option(start_speed, mid_speed, end_speed).map(|x| {
                        dwell_threshold(x, LONG_DWELL_SPEED_THRESH_MPS)
                    })
                })
                .flatten(),
        )
    }
    out
}

fn min_option(a: Option<f64>, b: Option<f64>, c: Option<f64>) -> Option<f64> {
    // filter out Nones and find the minimum value
    [a, b, c]
        .into_iter()
        .flatten()
        .min_by(|x, y| x.partial_cmp(y).unwrap())
}
