use serde::{Deserialize, Serialize};

use crate::{BackState, FrontState, LngLat};

/// Messages from the frontend to the backend over the websocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToBack {
    // Frontend requests frontend state at runtime
    GetFrontState,
    // And periodically requests the backend state
    GetBackState,
    // Set a new value for the UI/Persistent State (boxed to reduce enum size)
    SetFrontState(Box<FrontState>),
    GetPopupText((LngLat, Option<String>)),

    ReviewedLastError,

    RequestWhenInUseAuthorization,
    ExportSqliteLog,
    ImportSqliteLog,
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToFront {
    FrontState(Option<FrontState>),
    BackState(BackState),
    GeojsonUpdated,
    PopupText {
        location: LngLat,
        text: String,
        bg_color: String,
    },
}
