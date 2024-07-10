use serde::{Deserialize, Serialize};

use crate::{
    pin::Pin, state::DerivedState, BackState, FrontState, LngLat, Location,
};

/// Messages from the frontend to the backend over the websocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToBack {
    // Frontend requests frontend state at runtime
    GetFrontState,
    // And periodically requests the backend state
    GetBackState,
    GetDerivedState,
    // Set a new value for the UI/Persistent State (boxed to reduce enum size)
    SetFrontState(Box<FrontState>),
    GetLocationNear(LngLat),

    SavePin(Pin),
    DeletePin(i64), // database index

    DeleteSelectedLocations,

    ReviewedLastError,

    RequestWhenInUseAuthorization,
    GoToLocationSettings,
    ExportSqliteLog,
    ImportSqliteLog,
    ExportTrack,
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    FrontState(Box<Option<FrontState>>),
    BackState(BackState),
    DerivedState(DerivedState),
    GeojsonUpdated,
    NearestLocation(Location),
    NewPinId(i64),              // id of a new pin after assignment
    DeleteLocationsResult(u64), // how many points deleted
    SwiftPoke,
}
