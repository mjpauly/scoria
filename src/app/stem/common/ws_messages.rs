use serde::{Deserialize, Serialize};

use crate::{BackState, FrontState, Location, TimeRange};

/// Messages from the frontend to the backend over the websocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToBack {
    // Frontend requests frontend state at runtime
    GetFrontState,
    // And periodically requests the backend state
    GetBackState,
    // Set a new value for the UI/Persistent State
    SetFrontState(FrontState),
    GetLocationTimeRange(TimeRange),
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    FrontState(Option<FrontState>),
    BackState(BackState),
    // Send the last known location for displaying. Used to push new location
    // updates in real time
    LastLocation(Location),
    LocationTimeRange(TimeRange, Vec<Location>),
}
