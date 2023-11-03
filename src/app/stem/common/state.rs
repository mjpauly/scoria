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

/// Pick the type's default if it fails to deserialize. This ensures that an
/// error during deserialization doesn't cause the whole thing to fail.
/// `#[serde(default)]` on containers only picks the default if a field is
/// missing, not if there's an error deserializing. This is important for cases
/// when, for example, the name of an enum variant changes between app versions.
pub fn ok_or_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: serde::Deserializer<'de>,
{
    T::deserialize(d).or_else(|_| Ok(T::default()))
}

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
// Missing fields are filled in by the struct returned by the default
#[serde(default)]
pub struct BackState {
    #[serde(deserialize_with = "ok_or_default")]
    pub auto_location_config: AutoConfig,

    #[serde(deserialize_with = "ok_or_default")]
    pub last_location: Option<Location>,
    #[serde(deserialize_with = "ok_or_default")]
    pub locations_past_hour: Option<i32>,

    // data-derived state for the map view
    #[serde(deserialize_with = "ok_or_default")]
    pub data_center: Option<(LngLat, f64)>, // (LngLat, zoom)
    #[serde(deserialize_with = "ok_or_default")]
    pub cmap_params: CmapParams,

    // time of last data point where automap was updated
    #[serde(deserialize_with = "ok_or_default")]
    pub last_automap_update: LastAutomapUpdate,
    // Number of data points in the database which have not yet been
    // incorporated into the automap
    #[serde(deserialize_with = "ok_or_default")]
    pub num_automap_records_remaining: u64,

    // current size of the map cache
    #[serde(deserialize_with = "ok_or_default")]
    pub map_cache_size: u64,

    // the most recent error that was recorded, if it exists, and whether it was
    // reviewed by the user already (if yes, no prompting to review it)
    #[serde(deserialize_with = "ok_or_default")]
    pub last_logged_error: Option<(String, bool)>,
}

/// Driven by frontend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FrontState {
    // Last version of the introduction/tutorial that was viewed
    #[serde(deserialize_with = "ok_or_default")]
    pub last_viewed_intro_version: u32,

    #[serde(deserialize_with = "ok_or_default")]
    pub route: PersistedRoute,
    #[serde(deserialize_with = "ok_or_default")]
    pub settings_route: PersistedSettingsRoute,

    // Location configuration
    #[serde(deserialize_with = "ok_or_default")]
    pub location_config: UserConfig,

    // State of the plot view
    #[serde(deserialize_with = "ok_or_default")]
    pub map: MapState,

    // User's preferred display units
    #[serde(deserialize_with = "ok_or_default")]
    pub unit_pref: UnitPreference,

    // Map data cache preferences
    #[serde(deserialize_with = "ok_or_default")]
    pub map_cache_pref: MapCachePreference,

    // State of partially-filled problem report
    #[serde(deserialize_with = "ok_or_default")]
    pub problem_report: ProblemReport,
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
    MapSettings,
    ReportProblem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MapState {
    // time_delta_range is the persisted source of truth on the time range to
    // display, but time-fixed time_range is used to get data to plot, and is
    // updated only on particular actions, like an explicit time update, or when
    // the app is opened. This way the range is fixed while the app is open, but
    // updates to the delta range on launch.
    #[serde(deserialize_with = "ok_or_default")]
    pub time_delta_range: TimeDeltaRange,
    #[serde(deserialize_with = "ok_or_default")]
    pub time_range: TimeRange,
    #[serde(deserialize_with = "ok_or_default")]
    pub style: MapStyle,
    #[serde(deserialize_with = "ok_or_default")]
    pub filters: Vec<Filter>,
    #[serde(deserialize_with = "ok_or_default")]
    pub view_pos: ViewPosition,
}

/// Default for the backend to use if deserializing from file fails. The
/// times in time_range are not timezone aware, so will look wrong, but this is
/// just a backup.
impl Default for MapState {
    fn default() -> Self {
        Self {
            time_delta_range: Default::default(),
            time_range: Default::default(),
            style: Default::default(),
            filters: default_accuracy_filter(),
            view_pos: Default::default(),
        }
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LastAutomapUpdate(pub time::OffsetDateTime);

impl Default for LastAutomapUpdate {
    fn default() -> Self {
        Self(time::OffsetDateTime::from_unix_timestamp(0).unwrap())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapCachePreference {
    // max cache size in bytes, beyond which eviction happens.
    pub max_size: u64,

    // prevents loading map data from the internet, relying only on the cache
    pub disable_fetch: bool,
}

impl Default for MapCachePreference {
    fn default() -> Self {
        Self {
            max_size: 100 * 1000 * 1000, // 100 MB default
            disable_fetch: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProblemReport {
    #[serde(deserialize_with = "ok_or_default")]
    pub email: String,
    #[serde(deserialize_with = "ok_or_default")]
    pub body: String,
    #[serde(deserialize_with = "ok_or_default")]
    pub attach_log: bool,
}

impl Default for ProblemReport {
    fn default() -> Self {
        Self {
            email: String::from(""),
            body: String::from(""),
            attach_log: true,
        }
    }
}
