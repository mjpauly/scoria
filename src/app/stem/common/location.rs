use serde::{Deserialize, Serialize};

/// Struct representation of a Location data point
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub timestamp: time::OffsetDateTime,

    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,

    pub msl_altitude: Option<f64>,
    pub ellipsoid_altitude: Option<f64>,
    pub vertical_accuracy: Option<f64>,
    pub story: Option<i64>,

    pub speed: Option<f64>,
    pub speed_accuracy: Option<f64>,

    pub course: Option<f64>,
    pub course_accuracy: Option<f64>,

    pub is_simulated_by_software: Option<bool>,
    pub is_produced_by_accessory: Option<bool>,
}
