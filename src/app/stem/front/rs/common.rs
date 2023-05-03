//! Types common to the front and back ends
//!
//! Messages are serialized with bincode.
//!
//! # Contracts
//!
//! Any ToBack::{Get.., Set..} messages are to have the state
//! re-broadcasted by the backend to the UI. This way a Set message can change
//! the backend state and another component can be notified of the new state.

use serde::{Deserialize, Serialize};

/// Messages from the frontend to the backend over the websocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToBack {
    // Get all state values (LocationEnabled, LastLocation, LocationsPastHour,
    GetState,
    SetLocationConfig(LocationConfig),
    GetLocationTimeRange(TimeRange),
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    LocationConfig(LocationConfig),
    // Send the last known location for displaying.
    // Used to push new location updates in real time
    LastLocation(Location),
    // Number of location records recorded in the past hour
    LocationsPastHour(i32),
    LocationTimeRange(TimeRange, Vec<Location>),
}

/// Struct representation of a Location data point
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
    pub datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}

/// Location configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(default)]
pub struct LocationConfig {
    pub enabled: bool,
    pub mode: LocationMode,
    pub standard_config: StandardLocationConfig,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: LocationMode::Standard,
            standard_config: StandardLocationConfig {
                accuracy_mode: LocationAccuracyMode::Best,
                distance_filter: 5.0,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LocationMode {
    Standard,
    SignificantChanges,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StandardLocationConfig {
    pub accuracy_mode: LocationAccuracyMode,
    pub distance_filter: f32,
}

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Copy)]
pub enum LocationAccuracyMode {
    Best,
    TenMeters,
    HundredMeters,
    Kilometer,
    ThreeKilometers,
}

/// A range of times.
/// Encoding as a struct helps ensure `start` and `end` are not accidentally
/// swapped.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: time::OffsetDateTime,
    pub end: time::OffsetDateTime,
}

impl TimeRange {
    pub fn contains(&self, timestamp: &time::OffsetDateTime) -> bool {
        &self.start <= timestamp && timestamp < &self.end
    }
}
