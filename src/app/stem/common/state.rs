//! State that is mutually intelligible between the frontend and the
//! backend, and is saved between app launches
//!
//! We split this state in two to ensure we don't encounter race conditions when
//! the frontend and the backend both update the state at the same time. The
//! FrontState is driven by the frontend, and the BackState is driven by the
//! backend.

use serde::{Deserialize, Serialize};

use crate::{
    filters::Filter, map_style::MapStyle, view_position::ViewPosition,
    AutoConfig, Location, TimeRange, UserConfig,
};

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackState {
    pub auto_location_config: AutoConfig,

    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,
}

/// Driven by frontend
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontState {
    pub route: PersistedRoute,

    // Location configuration
    pub location_config: UserConfig,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: bool,

    // State of the plot view
    pub map: MapState,
}

/// The page the frontend is on. Only variants that we care to persist between
/// launches are stored.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedRoute {
    #[default]
    Sense,
    Analyze,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapState {
    pub time_range: TimeRange,
    pub style: MapStyle,
    pub filters: Vec<Filter>,
    pub view_pos: ViewPosition,
}
