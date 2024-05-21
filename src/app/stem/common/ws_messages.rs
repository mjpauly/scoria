use serde::{Deserialize, Serialize};

use crate::{pin::Pin, state::DerivedState, BackState, FrontState, LngLat};

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
    GetPopupText((LngLat, Option<String>)),

    SavePin(Pin),
    DeletePin(i64), // database index

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
    FrontState(Option<FrontState>),
    BackState(BackState),
    DerivedState(DerivedState),
    GeojsonUpdated,
    PopupText {
        location: LngLat,
        text: String,
        bg_color: String,
    },
    NewPinId(i64), // id of a new pin after assignment
    SwiftPoke,
}
