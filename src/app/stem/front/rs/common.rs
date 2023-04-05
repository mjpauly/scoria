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
    // (TODO: DistFilt))
    GetState,
    SetLocationEnabled(bool), // set location enabled state
    SetDistFilt(f32),         // distance filter
    GetLocationTimeRange(TimeRange),
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    LocationEnabled(bool),
    // Send the last known location for displaying.
    // Used to push new location updates
    LastLocation(Location),
    // Number of location records recorded in the past hour
    LocationsPastHour(i32), // TODO: roll this in will LastLocation?
    // Minimum horizontal distance in meters before a new data point is output
    DistFilt(f32),
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
