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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    cmaps::CmapParams,
    dashboard_metrics::DashboardMetrics,
    export_options::ExportOptions,
    filters::{DataStream, Filter, FilterOp},
    map_style::MapStyle,
    pin::Pin,
    plot_data::TimeSeriesPlot,
    time_range::TimeDeltaRange,
    timeline::Timeline,
    units::{time::TimePreference, UnitPreference},
    view_position::ViewPosition,
    AutoConfig, LngLat, Location, TimeRange, UserConfig,
};

/// Pick the type's default if it fails to deserialize.
///
/// This ensures that an error deserializing an inner field value doesn't cause
/// the whole deserialization to fail. This is useful, for example, when the
/// name of an enum variant has changed, but the app is reading old data.
///
/// Warning: it is not sufficient only annotate an outer type. Each enum must be
/// annotated with `ok_or_default`, since an annotation on the containing struct
/// will fail to catch the error and cause the entire deserialization to fail.
pub fn ok_or_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: serde::Deserializer<'de>,
{
    Ok(T::deserialize(d).unwrap_or_default())
}

/// State driven by the backend which is derived from other state, like the
/// database.
///
/// Does not get persisted between app restarts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DerivedState {
    // User-created map pins.
    pub pins: Vec<Pin>,
    pub dashboard_metrics: DashboardMetrics,
    pub colored_timeseries_plot: TimeSeriesPlot,
    pub timeline: Timeline,
}

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BackState {
    #[serde(deserialize_with = "ok_or_default")]
    pub app_version: String,
    // Next two fields only for Android. app_version_code is set at startup, and
    // available_app_version is Some if an upgrade exists
    #[serde(deserialize_with = "ok_or_default")]
    pub app_version_code: Option<i64>,
    #[serde(deserialize_with = "ok_or_default")]
    pub available_app_version: Option<(i64, String)>,

    #[serde(deserialize_with = "ok_or_default")]
    pub auto_location_config: AutoConfig,

    #[serde(deserialize_with = "ok_or_default")]
    pub last_location: Option<Location>,
    #[serde(deserialize_with = "ok_or_default")]
    pub locations_past_minute: Option<i32>,
    #[serde(deserialize_with = "ok_or_default")]
    pub locations_past_five_minutes: Option<i32>,

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
    // (Android) The latest version code that the user has reviewed for update
    #[serde(deserialize_with = "ok_or_default")]
    pub update_version_code_reviewed: i64,

    // Last version of the introduction/tutorial that was viewed
    #[serde(deserialize_with = "ok_or_default")]
    pub last_viewed_intro_version: u32,
    // Page of the introduction/tutorial that was viewed
    #[serde(deserialize_with = "ok_or_default")]
    pub intro_page: u32,

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

    // User's preferred display units
    #[serde(deserialize_with = "ok_or_default")]
    pub time_pref: TimePreference,

    // Map data cache preferences
    #[serde(deserialize_with = "ok_or_default")]
    pub map_cache_pref: MapCachePreference,

    // State of partially-filled problem report
    #[serde(deserialize_with = "ok_or_default")]
    pub problem_report: ProblemReport,

    // Export options for GPX, GeoJSON, CSV, etc
    #[serde(deserialize_with = "ok_or_default")]
    pub export_opts: ExportOptions,

    // Positions of scrolls, identified by a string ID.
    #[serde(deserialize_with = "ok_or_default")]
    pub scroll_positions: HashMap<String, i32>,

    // Points that are selected
    #[serde(deserialize_with = "ok_or_default")]
    pub selected_points: Vec<(time::OffsetDateTime, LngLat)>,
}

/// The page the frontend is on. Only variants that we care to persist between
/// launches are stored.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedRoute {
    #[default]
    Sense,
    Analyze,
    Places,
    Metrics,
    SettingsRoot,
    SettingsSubpage,
    Intro,
}

/// The settings page the frontend is on
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedSettingsRoute {
    #[default]
    Root,
    General,
    MapSettings,
    Export,
    Data,
    ReportProblem,
    Update,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MapState {
    // Map settings tab that's open (if any)
    #[serde(deserialize_with = "ok_or_default")]
    pub settings_tab: MapSettingsTab,
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
    // Pin that's being viewed/edited, but might not be saved.
    #[serde(deserialize_with = "ok_or_default")]
    pub current_pin: Pin,
    // Whether we're in edit mode for the pin or not
    #[serde(deserialize_with = "ok_or_default")]
    pub editable_pin: bool,
    // id of the pin that was last selected on the map (may not be the same as
    // the current pin in the PinEditor)
    #[serde(deserialize_with = "ok_or_default")]
    pub selected_pin_id: Option<i64>,
    #[serde(deserialize_with = "ok_or_default")]
    pub open_in_google_maps: bool,
    #[serde(deserialize_with = "ok_or_default")]
    pub popup_color: Option<String>, // color of popup background to display
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum MapSettingsTab {
    #[default]
    None,
    Filters,
    MapStyle,
    TimeRange,
    PinDetails,
    TimeSeriesPlot,
    SelectPoints,
}

/// Default for the backend to use if deserializing from file fails. The
/// times in time_range are not timezone aware, so will look wrong, but this is
/// just a backup.
impl Default for MapState {
    fn default() -> Self {
        Self {
            settings_tab: Default::default(),
            time_delta_range: Default::default(),
            time_range: Default::default(),
            style: Default::default(),
            filters: default_accuracy_filter(),
            view_pos: Default::default(),
            current_pin: Default::default(),
            editable_pin: Default::default(),
            selected_pin_id: Default::default(),
            open_in_google_maps: Default::default(),
            popup_color: Default::default(),
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LastAutomapUpdate(pub time::OffsetDateTime);

impl Default for LastAutomapUpdate {
    fn default() -> Self {
        Self(time::OffsetDateTime::from_unix_timestamp(0).unwrap())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapCachePreference {
    // max cache size in bytes, beyond which eviction happens.
    pub max_size: u64,

    // prevents loading map data from the internet, relying only on the cache
    pub disable_fetch: bool,
}

impl Default for MapCachePreference {
    fn default() -> Self {
        Self {
            max_size: 200 * 1000 * 1000, // 200 MB default
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
