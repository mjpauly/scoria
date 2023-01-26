//! Handles database access and modification.

use crate::paths;


/// Log a location event in the database.
pub fn log_location(lat: f64, lon: f64, accuracy: f64, speed: f64, course: f64)
    {
    // arguments are received as 64-bit float, only lat/lon can't be
    // truncated to 32-bit without (potentially) losing some accuracy,
    // truncation should happen before storing in DB
    // TODO: figure out time
    println!(
        "{:?}, {:?}, {:?}, {:?}, {:?}",
        lat, lon, accuracy, speed, course
    );
}
