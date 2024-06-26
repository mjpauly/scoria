//! Dwell detection
//!
//! Individual location points are annotated with whether they are part of a
//! dwell. When considering inter-point spans, a span is still considered part
//! of the dwell if just one of its two bounding points is part of the dwell.
//!
//! dwell               x   x       x
//! non-dwell   x   x                     x   x
//! all times   |   |                     |   |   (segmentation)
//!
use common::{LngLat, Location};
use itertools::Itertools;
use nav_types::WGS84;
use tracing::{instrument, Level};

use super::{
    distance::straight_distance, speed::avg_speed, stats::weighted_mean,
};

/// Short dwell detection speed (0.5 m/s = 1.8 km/h = 1.1 mi/h)
const SHORT_DWELL_SPEED_THRESH_MPS: f64 = 0.5;

/// Detect if a dwell exists between two locations.
pub fn short_dwell_detect(a: &Location, b: &Location) -> bool {
    avg_speed(a, b) < SHORT_DWELL_SPEED_THRESH_MPS
}

/// Segment a string of locations into a the continous segments without jumps at
/// the edge of the map view bounds.
///
/// bool is if the point is visible.
pub fn segment_on_visibility<'a>(
    records: &[(&'a Location, bool)],
) -> Vec<(bool, Vec<&'a Location>)> {
    let chunker = records.iter().chunk_by(|(_, d)| *d);
    chunker
        .into_iter()
        .map(|(k, c)| (k, c.map(|(l, _)| *l).collect::<Vec<_>>()))
        .collect::<Vec<_>>()
}

/// Score dwells after doing visibility segmentation.
pub fn segmented_dwell_scores(
    segments: &[(bool, Vec<&Location>)],
) -> Vec<Option<f64>> {
    let mut out = vec![];
    for (visible, seg) in segments.iter() {
        if *visible {
            out.extend_from_slice(&dwell_score(seg));
        } else {
            out.extend((0..seg.len()).map(|_| None));
        }
    }
    out
}

const LONG_DWELL_WIDTH_SECS: u64 = 120;
const OUTLIER_Z_SCORE: f64 = 2.5;

/// Slide a window at least 2 minutes long, and count all points a dwell if
/// standard deviation is small enough.
///
/// Must be a continuous segment of data points (no jumps due to view bounds
/// edges).
#[instrument(skip_all, level = Level::TRACE)]
pub fn dwell_score(records: &[&Location]) -> Vec<Option<f64>> {
    let mut min_stds = vec![Option::<f64>::None; records.len()];
    let mut ei = 0; // end index
    let dur_gte_big_thresh = |start: &Location, end: &Location| {
        end.timestamp - start.timestamp
            > time::Duration::seconds(LONG_DWELL_WIDTH_SECS as i64)
    };
    for si in 0..records.len() {
        // println!("{si}");
        while ei < records.len() - 1
            && !dur_gte_big_thresh(records[si], records[ei])
        {
            // while window is not larger than threshold size, advance end index
            ei += 1;
        }
        if !dur_gte_big_thresh(records[si], records[ei]) {
            // at the end; sliding window is no longer larger than min size
            break;
        }
        // weights are the time spent at the point (duration until next point)
        let lnglats_and_weights = records[si..ei + 1]
            .iter()
            .tuple_windows::<(_, _)>()
            .map(|(a, b)| {
                (a.lnglat(), (b.timestamp - a.timestamp).as_seconds_f64())
            });
        let (_, stddev) = weighted_lnglat_mean_and_stddev_without_outliers(
            lnglats_and_weights.clone(),
            Some(OUTLIER_Z_SCORE),
        );
        for min_std in min_stds.iter_mut().take(ei).skip(si) {
            // index the range si..ei
            if let Some(prev) = min_std {
                *min_std = Some(prev.min(stddev));
            } else {
                *min_std = Some(stddev);
            }
        }
    }
    min_stds
}

/// The outlier z score determines which points should be removed from the mean
/// and std calculation.
pub fn weighted_lnglat_mean_and_stddev_without_outliers(
    lnglats_and_weights: impl Iterator<Item = (LngLat, f64)> + Clone,
    outlier_z_score: Option<f64>,
) -> (LngLat, f64) {
    let (mut center, mut stddev, z_scores) =
        weighted_lnglat_mean_and_stddev(lnglats_and_weights.clone());
    if let Some(outlier_threshold) = outlier_z_score {
        let (newcenter, newstddev, _) = weighted_lnglat_mean_and_stddev(
            lnglats_and_weights
                .zip(z_scores)
                .filter_map(|(llw, z)| (z < outlier_threshold).then_some(llw)),
        );
        center = newcenter;
        stddev = newstddev;
    }
    (center, stddev)
}

/// Get the weighted mean location and the standard deviation of the distances
/// to that location.
///
/// Also returns an iterator over the the z scores for each location.
pub fn weighted_lnglat_mean_and_stddev(
    lnglats_and_weights: impl Iterator<Item = (LngLat, f64)> + Clone,
) -> (LngLat, f64, impl Iterator<Item = f64> + Clone) {
    let lng =
        weighted_mean(lnglats_and_weights.clone().map(|(ll, w)| (ll.lng, w)))
            .unwrap_or(0.0);
    let lat =
        weighted_mean(lnglats_and_weights.clone().map(|(ll, w)| (ll.lat, w)))
            .unwrap_or(0.0);
    let center = WGS84::from_degrees_and_meters(lat, lng, 0.0);
    let squared_distances_and_weights =
        lnglats_and_weights.map(move |(ll, w)| {
            let pos = WGS84::from_degrees_and_meters(ll.lat, ll.lng, 0.0);
            (straight_distance(&center, &pos).powi(2), w)
        });
    let stddev = weighted_mean(squared_distances_and_weights.clone())
        .unwrap_or(0.0)
        .sqrt();
    let z_scores =
        squared_distances_and_weights.map(move |(d, _)| d.sqrt() / stddev);
    (LngLat { lng, lat }, stddev, z_scores)
}

const LONG_DWELL_THRESHOLD: f64 = 10.0;

/// Returns true for records that are part of a dwell.
pub fn long_dwell_threshold<'a>(
    scores: impl Iterator<Item = &'a Option<f64>>,
) -> Vec<bool> {
    scores
        .map(|ds| ds.map(|v| v <= LONG_DWELL_THRESHOLD).unwrap_or(false))
        .collect()
}
/*
#[instrument(skip_all, level = Level::TRACE)]
pub fn merge_close_dwells(records: &mut [(&Location, Option<bool>)]) {
    println!("");
    let chunker = records.iter_mut().chunk_by(|(_, d)| *d);
    let mut chunks = chunker
        .into_iter()
        .map(|(_k, c)| c.collect::<Vec<_>>())
        .collect::<Vec<_>>();
    if chunks.len() >= 3 {
        for i in 1..chunks.len() - 1 {
            // a and c are the dwell points that straddle the non-dwell
            let a = chunks[i - 1].last().unwrap(); // chunks have >=1 value
            let c = chunks[i + 1].first().unwrap();
            if a.1 == Some(true)
                && c.1 == Some(true)
                && c.0.timestamp - a.0.timestamp
                    < time::Duration::seconds(LONG_DWELL_WIDTH_SECS as i64)
                && distance_between_lla(
                    &dbg!(center_of_locations(
                        chunks[i - 1].iter().map(|(l, _)| *l)
                    )),
                    &dbg!(center_of_locations(
                        chunks[i - 1].iter().map(|(l, _)| *l)
                    )),
                ) < 10.0
            {
                for b in &mut chunks[i] {
                    if let Some(ref mut is_dwell) = b.1 {
                        *is_dwell = true;
                    }
                }
            }
        }
    }
}
*/
