use common::Location;
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

const LONG_DWELL_WIDTH_SECS: u64 = 120;
const OUTLIER_Z_SCORE: f64 = 2.5;

/// Slide a window at least 2 minutes long, and count all points a dwell if
/// standard deviation is small enough.
#[instrument(skip_all, level = Level::TRACE)]
pub fn dwell_score(records: &[(&Location, bool)]) -> Vec<Option<f64>> {
    let mut min_stds = vec![Option::<f64>::None; records.len()];
    let mut ei = 0; // end index
    let dur_gte_big_thresh = |start: &Location, end: &Location| {
        end.timestamp - start.timestamp
            > time::Duration::seconds(LONG_DWELL_WIDTH_SECS as i64)
    };
    for si in 0..records.len() {
        // println!("{si}");
        while ei < records.len() - 1
            && !dur_gte_big_thresh(records[si].0, records[ei].0)
        {
            // while window is not larger than threshold size, advance end index
            ei += 1;
        }
        if !dur_gte_big_thresh(records[si].0, records[ei].0) {
            // at the end; sliding window is no longer larger than min size
            break;
        }
        // TODO: only contiguous segments
        // weights are the time spent at the point (duration until next point)
        let lnglats_and_weights = records[si..ei + 1]
            .iter()
            .map(|(l, _)| l)
            .tuple_windows::<(_, _)>()
            .map(|(a, b)| {
                (a.lnglat(), (b.timestamp - a.timestamp).as_seconds_f64())
            });
        let lng_mean = weighted_mean(
            lnglats_and_weights.clone().map(|(ll, w)| (ll.lng, w)),
        )
        .unwrap_or(0.0);
        let lat_mean = weighted_mean(
            lnglats_and_weights.clone().map(|(ll, w)| (ll.lat, w)),
        )
        .unwrap_or(0.0);
        let center = WGS84::from_degrees_and_meters(lat_mean, lng_mean, 0.0);
        let squared_distances_and_weights =
            lnglats_and_weights.clone().map(|(ll, w)| {
                let pos = WGS84::from_degrees_and_meters(ll.lat, ll.lng, 0.0);
                (straight_distance(&center, &pos).powi(2), w)
            });
        let mut stddev = weighted_mean(squared_distances_and_weights.clone())
            .unwrap_or(0.0)
            .sqrt();
        let without_outliers = squared_distances_and_weights
            .filter(|(d, _)| d.sqrt() / stddev < OUTLIER_Z_SCORE);
        stddev = weighted_mean(without_outliers).unwrap_or(0.0).sqrt();
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
