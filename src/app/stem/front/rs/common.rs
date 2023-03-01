//! Types common to the front and back ends
//!
//! Messages are serialized with rmp_serde in the MessagePack binary format.

use serde::{Deserialize, Serialize};

/// Messages from the frontend to the backend over the websocket
#[derive(Serialize, Deserialize)]
pub enum MsgForBackend {
    GetLocationEnabled,       // get location enabled state
    SetLocationEnabled(bool), // set location enabled state
}

/// Messages from the backend to the frontend
#[derive(Serialize, Deserialize)]
pub enum MsgForFrontend {
    NewLocationData(Location), // send new location data for displaying
}

/// Struct representation of a Location data point
#[derive(Clone, Debug)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
    pub datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}
