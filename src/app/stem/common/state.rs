//! State that is mutually intelligible between the frontend and the
//! backend, and is saved between app launches
//!
//! We split this state in two to ensure we don't encounter race conditions when
//! the frontend and the backend both update the state at the same time. The
//! FrontState is driven by the frontend, and the BackState is driven by the
//! backend.
//!
//! #[serde(default)] is put on structs to indicate that missing fields are to
//! be pulled from the type's default implementation.

use serde::{Deserialize, Serialize};

use crate::{
    filters::{DataStream, Filter, FilterOp},
    map_style::MapStyle,
    view_position::ViewPosition,
    AutoConfig, Location, TimeRange, UserConfig,
};

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
// Missing fields are filled in by the struct returned by the default
#[serde(default)]
pub struct BackState {
    pub auto_location_config: AutoConfig,

    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,
}

/// Driven by frontend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
pub struct MapState {
    pub time_range: TimeRange,
    pub style: MapStyle,
    pub filters: Vec<Filter>,
    pub view_pos: ViewPosition,
}

/// Default for the backend to use if deserializing from file fails. The
/// times in time_range are not timezone aware, so will look wrong, but this is
/// just a backup.
impl Default for MapState {
    fn default() -> Self {
        Self {
            time_range: plus_minus_day_utc(),
            style: Default::default(),
            filters: default_accuracy_filter(),
            view_pos: Default::default(),
        }
    }
}

/// Time range to use if it can't be parsed from file. Does not depend on the
/// timezone offset, so it is safe to be run by the backend. This is a backup
/// in case deserialization doesn't work. On first install the frontend should
/// be what initializes the time_range.
pub fn plus_minus_day_utc() -> TimeRange {
    let now = time::OffsetDateTime::now_utc();
    let start = now - time::Duration::DAY;
    let end = now + time::Duration::DAY;
    TimeRange { start, end }
}

pub fn default_accuracy_filter() -> Vec<Filter> {
    vec![Filter {
        id: 0,
        enabled: true,
        datastream: DataStream::HorizAccuracy,
        op: FilterOp::GreaterThan,
        threshold: 10.0,
    }]
}
