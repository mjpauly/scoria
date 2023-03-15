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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToBack {
    GetState,                 // get all state values
    SetLocationEnabled(bool), // set location enabled state
    SetDistFilt(f32),         // distance filter
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToFront {
    // Send the last known location for displaying.
    // Used to push new location updates
    LastLocation(Location),
    // Number of location records recorded in the past hour
    LocationsPastHour(i32), // TODO: roll this in will LastLocation?
    LocationEnabled(bool),
    // Minimum horizontal distance in meters before a new data point is output
    DistFilt(f32),
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
