//! Test that the 2d difference operation is well behaved on an input which is
//! known to be problematic for geo.
//!
//! bazel test tests/geo_diff:diff_test --test_output=all --sandbox_debug=true --cache_test_results=no -c opt
//! Try with `-c opt` for the indefinite hang.

// use geo::BooleanOps;
use geo::{Coord, LineString, MapCoordsInPlace, MultiPolygon, Polygon};
use geo_clipper::Clipper;

// const PREFIX: &str = "src/app/stem/tests/geo_diff";

// BoolOp precision (scale factor to increase before casting as integer)
const BOOL_OP_SCALE_FACTOR: f64 = 4096.0;

// /// Captures a test case where geo::BooleanOps would hang indefinitely, if
// /// compiled with optimizations, and panic if not.
// #[test]
// fn no_indefinite_hang() {
// println!("Run dir: {}", std::env::current_dir().unwrap().display());
// let left_path = format!("{PREFIX}/left.bin");
// let right_path = format!("{PREFIX}/right.bin");
// let left: MultiPolygon =
// bincode::deserialize(&std::fs::read(left_path).unwrap()).unwrap();
// let right: MultiPolygon =
// bincode::deserialize(&std::fs::read(right_path).unwrap()).unwrap();
//
// Clipper::difference(&left, &right, BOOL_OP_SCALE_FACTOR);
//     // BooleanOps::difference(&left, &right);
// }

/// Test the behavior on a simpler case, which should panic and hang the same
/// way
#[test]
fn simple_no_panic() {
    let geo1 = Polygon::new(
        LineString(vec![
            Coord { x: -1.0, y: 46.0 },
            Coord { x: 8.0, y: 46.0 },
            Coord { x: 8.0, y: 39.0 },
            Coord { x: -1.0, y: 39.0 },
            Coord { x: -1.0, y: 46.0 },
        ]),
        vec![LineString(vec![
            Coord { x: 2.0, y: 45.0 },
            Coord { x: 7.0, y: 45.0 },
            Coord { x: 7.0, y: 44.0 },
            Coord { x: 5.0, y: 42.0 },
            Coord { x: 5.0, y: 41.0 },
            Coord { x: 0.0, y: 43.0 },
            Coord { x: 2.0, y: 45.0 },
        ])],
    );
    let geo2 = Polygon::new(
        LineString(vec![
            Coord { x: 0.0, y: 42.0 },
            Coord { x: 6.0, y: 44.0 },
            Coord { x: 4.0, y: 40.0 },
            Coord { x: 0.0, y: 42.0 },
        ]),
        vec![],
    );
    let mut left = MultiPolygon::new(vec![geo1]);
    let mut right = MultiPolygon::new(vec![geo2]);
    let shift = |c: Coord| Coord {
        x: c.x + 931230.,
        y: c.y + 412600.,
    };
    left.map_coords_in_place(shift);
    right.map_coords_in_place(shift);
    for i in 0..10 {
        println!("{} ", i);
        Clipper::difference(&left, &right, BOOL_OP_SCALE_FACTOR);
        // Usually need to run a second time for this to panic:
        // BooleanOps::difference(&left, &right);
    }
}

/// Test the behavior on a simple difference operation. Good for testing what
/// changing the scale factor does
#[test]
fn simple() {
    let left = MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord { x: 0., y: 0. },
            Coord { x: 0., y: 1. },
            Coord { x: 1., y: 1. },
            Coord { x: 1., y: 0. },
        ]),
        vec![],
    )]);
    let right = MultiPolygon::new(vec![Polygon::new(
        LineString::new(vec![
            Coord { x: 0.51, y: 0.51 },
            Coord { x: 0.51, y: 1. },
            Coord { x: 1., y: 1. },
            Coord { x: 1., y: 0.51 },
        ]),
        vec![],
    )]);
    let out = Clipper::difference(&left, &right, BOOL_OP_SCALE_FACTOR);
    dbg!(out);
    // panic!("let us see the output"); // uncomment to see output
}
