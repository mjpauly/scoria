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
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToFront {
    LastLocation(Location), // send the last known location for displaying.
    // used to push new location updates
    LocationEnabled(bool),
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
