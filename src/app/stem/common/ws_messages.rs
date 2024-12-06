use serde::{Deserialize, Serialize};

use crate::{
    mounted::MountID,
    pin::Pin,
    popups::PopUp,
    state::{DerivedState, PendingEvents},
    BackState, FrontState, LngLat, Location,
};

/// Messages from the frontend to the backend over the websocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToBack {
    LogError(String),

    // Frontend requests frontend state at startup, once
    GetStartupState,
    // And periodically requests the backend state
    GetBackState,
    // Set a new value for the UI/Persistent State (boxed to reduce enum size)
    SetFrontState(Box<FrontState>),
    GetLocationNear(MountID, LngLat),

    SavePin(Pin),
    DeletePin(i64), // database index

    DeleteSelectedLocations,
    CopySelectedLocationsToDatabase,

    ReviewedLastError,

    RequestWhenInUseAuthorization,
    GoToLocationSettings,
    ExportSqliteLog,
    ImportSqliteLog,
    ExportTrack,
    ImportPlaces,
    MountDB,
    DeleteMountedDB(MountID),
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    Startup(
        Box<Option<FrontState>>,
        BackState,
        DerivedState,
        Option<PendingEvents>,
    ),
    BackState(BackState),
    DerivedState(DerivedState),
    GeojsonUpdated,
    NearestLocation(MountID, Location, jiff::Zoned), // zoned datetime also sent
    NewPinId(i64), // id of a new pin after assignment
    SwiftPoke,
    PendingEvents(PendingEvents),
    PopUp(PopUp), // message from backend to display
}
