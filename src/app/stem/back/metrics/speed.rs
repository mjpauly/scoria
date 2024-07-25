//! Experimental query example, segmenting based on speed to differentiate
//! motion from dwells. Can be used to find time spent at destinations, time
//! spent in travel, extent to which stops during transit add to trip times.
//!
//! Segmentation methods:
//!     - avg speed between two data points above/below threshold
//!             - need to ensure distance between data points doesn't indicate
//!             that there was a jump in the data (100m threshold?)
//!             - speed attached to data points not a reliable way to find
//!             dwells, since if you're stopped there's no data because of the
//!             distance filter
//!             - for now just using straight-line distance and ignoring earth
//!             clipping, also assuming Best accuracy and not filtering

use super::distance::distance_between_locations;
use super::segmentation::threshold_segmentation;
use common::Location;

/// Segments a location LineString into two MultiLineStrings, one for the
/// LineStrings above the speed threshold and one for below. A LineString is a
/// contiguous segment of travel or stationarity.
///
/// Speed calculation is on the average speed between data points.
///
/// Returns two vectors of vectors containing references to location records,
/// the first for records above the threhsold, and the second below.
pub fn speed_segmentation<'a, I>(
    records: I,
    threshold: f64,
) -> (Vec<Vec<&'a Location>>, Vec<Vec<&'a Location>>)
where
    I: Iterator<Item = &'a Location>,
{
    threshold_segmentation(records, |(a, b)| avg_speed(a, b), threshold)
}

/// Return the average speed between two location records in m/s
pub fn avg_speed(first: &Location, second: &Location) -> f64 {
    let distance = distance_between_locations(first, second); // m
    let dt = (second.timestamp - first.timestamp).abs(); // s
    distance / dt.as_seconds_f64() // m/s
}

#[cfg(test)]
mod tests {
    use super::speed_segmentation;
    use crate::metrics::tests::new_empty_location;

    #[test]
    fn speed_segmentation_test() {
        let mut rec = new_empty_location();
        let mut recs = vec![];
        recs.push(rec.clone()); // 0

        rec.timestamp += time::Duration::seconds(1);
        recs.push(rec.clone()); // 0->1 stopped
        rec.timestamp += time::Duration::seconds(1);
        rec.longitude += 1.;
        recs.push(rec.clone()); // 1->2 moving
        rec.timestamp += time::Duration::seconds(1);
        rec.longitude += 1.;
        recs.push(rec.clone()); // 2->3 moving

        rec.timestamp += time::Duration::seconds(1);
        recs.push(rec.clone()); // 3->4 stopped

        rec.timestamp += time::Duration::seconds(1);
        rec.longitude += 1.;
        recs.push(rec.clone()); // 4->5 moving

        let recs = recs;

        let result = speed_segmentation(recs.iter(), 1.0).clone();

        let expected = (
            [
                [&recs[1], &recs[2], &recs[3]].to_vec(),
                [&recs[4], &recs[5]].to_vec(),
            ]
            .to_vec(),
            [[&recs[0], &recs[1]].to_vec(), [&recs[3], &recs[4]].to_vec()]
                .to_vec(),
        );
        let _a = recs[0..2].to_vec();
        assert_eq!(expected, result);
    }
}
