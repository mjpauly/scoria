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
    cmaps::CmapParams,
    filters::{DataStream, Filter, FilterOp},
    map_style::MapStyle,
    time_range::TimeDeltaRange,
    units::UnitPreference,
    view_position::ViewPosition,
    AutoConfig, LngLat, Location, TimeRange, UserConfig,
};

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
// Missing fields are filled in by the struct returned by the default
#[serde(default)]
pub struct BackState {
    pub auto_location_config: AutoConfig,

    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,

    // data-derived state for the map view
    pub data_center: Option<(LngLat, f64)>, // (LngLat, zoom)
    pub cmap_params: CmapParams,
}

/// Driven by frontend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FrontState {
    // Last version of the introduction/tutorial that was viewed
    pub last_viewed_intro_version: usize,

    pub route: PersistedRoute,
    pub settings_route: PersistedSettingsRoute,

    // Location configuration
    pub location_config: UserConfig,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: bool,

    // State of the plot view
    pub map: MapState,

    // User's preferred display units
    pub unit_pref: UnitPreference,
}

/// The page the frontend is on. Only variants that we care to persist between
/// launches are stored.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedRoute {
    #[default]
    Sense,
    Analyze,
    SettingsRoot,
    SettingsSubpage,
    Intro,
}

/// The settings page the frontend is on
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedSettingsRoute {
    #[default]
    Root,
    General,
    Data,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MapState {
    // time_delta_range is the persisted source of truth on the time range to
    // display, but time-fixed time_range is used to get data to plot, and is
    // updated only on particular actions, like an explicit time update, or when
    // the app is opened. This way the range is fixed while the app is open, but
    // updates to the delta range on launch.
    pub time_delta_range: TimeDeltaRange,
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
            time_delta_range: plus_minus_day_utc(),
            time_range: (&plus_minus_day_utc()).into(),
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
pub fn plus_minus_day_utc() -> TimeDeltaRange {
    TimeDeltaRange {
        start_offset: -time::Duration::DAY,
        end_offset: time::Duration::DAY,
        snap_start_to_day: false,
        snap_end_to_day: false,
        offset: None,
    }
}

pub fn default_accuracy_filter() -> Vec<Filter> {
    vec![Filter {
        id: 0,
        enabled: true,
        datastream: DataStream::HorizAccuracy,
        op: FilterOp::GreaterThan,
        threshold: 100.0,
    }]
}
