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

use coord_transforms::{geo::lla2ecef, structs::geo_ellipsoid};
use nalgebra::Vector3;

use common::Location;

// Threshold above which the distance on the curved surface of the earth should
// be used. At 0.5 radians (28.6 deg) the error introduced from using a straight
// line distance instead of great circle distance is 1%.
const CURVED_DISTANCE_THRESHOLD_RAD: f64 = 0.5;
const CURVED_DISTANCE_THRESHOLD_M: f64 =
    CURVED_DISTANCE_THRESHOLD_RAD * geo_ellipsoid::WGS84_SEMI_MAJOR_AXIS_METERS;

/// Compute the distance between two locations. If altitude data is available
/// for both locations, then it's used in the distance calculation. Otherwise
/// both are considered to be at zero altitude.
pub fn distance_between_locations(a: &Location, b: &Location) -> f64 {
    let (a_alt, b_alt) = match (a.ellipsoid_altitude, b.ellipsoid_altitude) {
        (Some(a_alt), Some(b_alt)) => (a_alt, b_alt),
        _ => (0., 0.),
    };
    let a_lla = location_to_lla(a, a_alt);
    let b_lla = location_to_lla(b, b_alt);
    // compute the straight distance, which is good enough as long as we
    // regularly have data
    let straight = straight_distance(&a_lla, &b_lla);
    // if this distance is greater than our straight/curved error threshold,
    // compute it with the great circle distance
    if straight < CURVED_DISTANCE_THRESHOLD_M {
        straight
    } else {
        great_circle_distance(&a_lla, &b_lla)
    }
}

/// Get the LLA vector (lat, lng, alt) in radians and meters from a location,
/// and using the supplied altitude.
fn location_to_lla(l: &Location, alt: f64) -> Vector3<f64> {
    Vector3::new(l.latitude.to_radians(), l.longitude.to_radians(), alt)
}

/// Computes the straight-line distance between two LLA (lat, lon, alt) points
/// given in *radians and meters*.
pub fn straight_distance(a_lla: &Vector3<f64>, b_lla: &Vector3<f64>) -> f64 {
    let ellipsoid = geo_ellipsoid::geo_ellipsoid::new(
        geo_ellipsoid::WGS84_SEMI_MAJOR_AXIS_METERS,
        geo_ellipsoid::WGS84_FLATTENING,
    );
    let a_ecef = lla2ecef(a_lla, &ellipsoid);
    let b_ecef = lla2ecef(b_lla, &ellipsoid);
    a_ecef.metric_distance(&b_ecef)
}

/// Compute the great circle distance between two points.
///
/// This is used if the central angle between two points is greater than 0.5
/// radians, and the error introduced from using the straight distance would be
/// greater than 1%
pub fn great_circle_distance(
    a_lla: &Vector3<f64>,
    b_lla: &Vector3<f64>,
) -> f64 {
    // first elem [0] is latittude, second [1] is longitude
    let central_angle = (a_lla[0].sin() * b_lla[0].sin()
        + a_lla[0].cos() * b_lla[0].cos() * (a_lla[1] - b_lla[1]).cos())
    .acos();
    geo_ellipsoid::WGS84_SEMI_MAJOR_AXIS_METERS * central_angle // arc length
}

#[cfg(test)]
mod tests {
    use nalgebra::Vector2;

    use super::*;

    #[test]
    fn straight_distance_test() {
        let a_lla = Vector3::new(0., 0., 0.);
        // 1 longitude degree to the east
        let b_lla = Vector3::new(0., (1.0_f64).to_radians(), 0.);

        // equatorial earth radius in m
        let re = geo_ellipsoid::WGS84_SEMI_MAJOR_AXIS_METERS;

        // slice equatorially
        let angle = b_lla[1];
        let a_plane_location = Vector2::new(re, 0.);
        let b_plane_location = Vector2::new(re * angle.cos(), re * angle.sin());
        let expected = a_plane_location.metric_distance(&b_plane_location);

        let computed = straight_distance(&a_lla, &b_lla);
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
            let a_lla = Vector3::new(0., 0., 0.);
            let b_lla = Vector3::new(0., rad, 0.);
            let straight = straight_distance(&a_lla, &b_lla);
            let curved = great_circle_distance(&a_lla, &b_lla);
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
