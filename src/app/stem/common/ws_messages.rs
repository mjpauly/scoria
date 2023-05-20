use serde::{Deserialize, Serialize};

use crate::{Location, LocationConfig, TimeRange};

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
