//! Query building-block which finds distances between data points.
//!
//! For short distances, a straight line is used in ECEF XYZ coordinates. This
//! is simple and correct in that it views vertical changes as movement.
//!
//! The only trouble is when there's a significant distance between points, like
//! if logging was off during a long-haul flight to the other side of the globe.
//! Since the user hasn't actually tunneled through the globe, the more correct
//! distance calculation is the great circle distance between the points. The
//! error between the two methods is (2 * Re) for straight line vs (tau/2 * Re)
//! for great-circle distance, or tau/4 = 57% error.

use nav_types::{ECEF, WGS84};

use common::Location;

// Threshold above which the distance on the curved surface of the earth should
// be used. At 0.5 radians (28.6 deg) the error introduced from using a straight
// line distance instead of great circle distance is 1%.
const CURVED_DISTANCE_THRESHOLD_RAD: f64 = 0.5;
const WGS84_SEMI_MAJOR_AXIS_METERS: f64 = 6378137.0;
const CURVED_DISTANCE_THRESHOLD_M: f64 =
    CURVED_DISTANCE_THRESHOLD_RAD * WGS84_SEMI_MAJOR_AXIS_METERS;

/// Compute the distance between two locations. If altitude data is available
/// for both locations, then it's used in the distance calculation. Otherwise
/// both are considered to be at zero altitude.
pub fn distance_between_locations(a: &Location, b: &Location) -> f64 {
    let (a_alt, b_alt) = match (a.ellipsoid_altitude, b.ellipsoid_altitude) {
        (Some(a_alt), Some(b_alt)) => (a_alt, b_alt),
        _ => (0., 0.),
    };
    let a_wgs84 = location_to_wgs84(a, a_alt);
    let b_wgs84 = location_to_wgs84(b, b_alt);
    distance_between_points(&a_wgs84, &b_wgs84)
}

/// Compute the distance between two points.
///
/// Uses the euclidian distance for small distances, and switches to the great
/// circle distance if the error would be greater than 1%.
pub fn distance_between_points(a: &WGS84<f64>, b: &WGS84<f64>) -> f64 {
    // compute the straight distance, which is accurate when points are close
    let straight = straight_distance(a, b);
    if straight < CURVED_DISTANCE_THRESHOLD_M {
        straight
    } else {
        // distance greater than our straight/curved error threshold, use great
        // circle distance
        a.distance(b)
    }
}

/// Get the WGS84 type for a Location, using the supplied altitude.
fn location_to_wgs84(l: &Location, alt: f64) -> WGS84<f64> {
    WGS84::from_degrees_and_meters(l.latitude, l.longitude, alt)
}

/// Computes the straight-line distance between two points.
pub fn straight_distance(a: &WGS84<f64>, b: &WGS84<f64>) -> f64 {
    let a_ecef = ECEF::from(*a);
    let b_ecef = ECEF::from(*b);
    a_ecef.distance(&b_ecef)
}

#[cfg(test)]
mod tests {
    use nalgebra::Vector2;

    use super::*;

    #[test]
    fn straight_distance_test() {
        let angle = (1.0_f64).to_radians();
        let a = WGS84::from_radians_and_meters(0., 0., 0.);
        // 1 longitude degree to the east
        let b = WGS84::from_radians_and_meters(0., angle, 0.);

        // equatorial earth radius in m
        let re = WGS84_SEMI_MAJOR_AXIS_METERS;

        // slice equatorially
        let a_plane_location = Vector2::new(re, 0.);
        let b_plane_location = Vector2::new(re * angle.cos(), re * angle.sin());
        let expected = a_plane_location.metric_distance(&b_plane_location);

        let computed = straight_distance(&a, &b);
        let difference = (computed - expected).abs();

        println!("expected: {}", expected);
        println!("computed: {}", computed);
        println!("difference: {}", difference);

        assert!(difference < 1.0); // error less than 1m
    }

    use std::f64::consts::TAU;
    #[test]
    fn great_circle_test() {
        // i is percent of a half turn around the globe
        for i in 1..100 {
            let rad = TAU / 2. * i as f64 / 100.;
            let a = WGS84::from_radians_and_meters(0., 0., 0.);
            let b = WGS84::from_radians_and_meters(0., rad, 0.);
            let straight = straight_distance(&a, &b);
            let curved = a.distance(&b);
            // curved is the true value, so we have error as a percentage of it
            let err = (curved - straight) / curved;
            println!(
                "rad, straight, curved, err, {}, {}, {}, {}",
                rad, straight, curved, err
            );
            // as defined, this threshold is (approximately) the point at which
            // the error becomes larger than 1%
            if rad < CURVED_DISTANCE_THRESHOLD_RAD {
                assert!(err < 0.01);
            } else {
                assert!(err > 0.01);
            }
        }
    }
}
