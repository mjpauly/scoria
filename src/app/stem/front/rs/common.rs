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
    SetLocationEnabled(bool), // set location enabled state
    SetDistFilt(f32),         // distance filter
    SetSignificantChanges(bool),
    SetLocationAccuracyMode(LocationAccuracyMode),
    GetLocationTimeRange(TimeRange),
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    // Location configuration state
    LocationEnabled(bool),
    DistFilt(f32),
    SignificantChanges(bool),
    LocationAccuracyMode(LocationAccuracyMode),
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
