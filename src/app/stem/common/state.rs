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

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::{
    cmaps::CmapParams,
    dashboard_metrics::DashboardMetrics,
    export_options::ExportOptions,
    filters::{DataStream, Filter, FilterOp},
    map_style::MapStyle,
    mounted::{MountID, MountedDB},
    notif_pref::NotificationPreference,
    pin::{Pin, PinSettings},
    plot_data::TimeSeriesPlot,
    time_range::{StepTarget, TimeDeltaRange, TimeStep, TZ},
    timeline::{Timeline, TimelineConfig},
    units::{time::TimePreference, UnitPreference},
    view_position::ViewPosition,
    AutoConfig, LngLat, Location, TimeRange, UserConfig,
};

/// Pick the type's default if it fails to deserialize from JSON, but error on
/// binary formats like bincode.
///
/// This ensures that an error deserializing an inner field value doesn't cause
/// the whole deserialization to fail. This is useful, for example, when the
/// name of an enum variant has changed, but the app is reading old data.
///
/// TODO: is this warning still true?
/// Warning: it is not sufficient only annotate an outer type. Each enum must be
/// annotated with `ok_or_default`, since an annotation on the containing struct
/// will fail to catch the error and cause the entire deserialization to fail.
pub fn ok_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: serde::Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        // is_human_readable corresponds to self-describing formats like json,
        // where we can use deserialize_any into a Value and consume the
        // incorrect type. if we don't consume it we get an error
        let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
        Ok(T::deserialize(v).unwrap_or_default())
    } else {
        // On binary formats we can't handle the incorrect type (e.g. if it's a
        // seq but we expected a string), since we need to know what the type is
        //
        // If we wanted, we could still get a default with
        //
        // ```
        // Ok(T::deserialize(deserializer).unwrap_or_default())
        // ```

        T::deserialize(deserializer)
    }
}

/// Where a database on disk is in being opened. Opening runs in the
/// background so the app launches while a large database migrates; queries
/// against a database run only once it is Ready.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DbStatus {
    /// What the open is doing, and the fraction done where that's known.
    Opening {
        stage: String,
        progress: Option<f32>,
    },
    Ready,
    Error(String),
}

impl DbStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, DbStatus::Ready)
    }
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
    pub colored_timeseries_plot: BTreeMap<MountID, TimeSeriesPlot>,
    pub timeline: Timeline,
    // databases on disk and their open status (contains the main database
    // at index 0)
    pub mounted_dbs_on_disk: BTreeMap<MountID, DbStatus>,
    pub last_mounted: Option<(MountID, String)>, // id + name of last mounted db
    // most recent hourly log file containing an error, if any
    pub last_logged_error: Option<LoggedError>,
}

/// Contents of an hourly log file in which an error was logged. Rebuilt from
/// disk rather than persisted, since it can be large.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoggedError {
    // file name, e.g. log.2026-09-02-03, which identifies the hour
    pub log_file: String,
    // the file contents, possibly truncated to the tail
    pub contents: String,
}

/// Events that accumulate before UI is active, but which are handled in the UI.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PendingEvents {
    pub opened_url: Option<String>, // a scoria:// url that was opened
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

    // name of the hourly log file (DerivedState::last_logged_error) the user
    // last reviewed. errors in a later file prompt the user to review again.
    #[serde(deserialize_with = "ok_or_default")]
    pub reviewed_error_log: Option<String>,

    // the tz to use for the map time range, using tz pref. if localized, uses
    // the tz in the center of the map
    #[serde(deserialize_with = "ok_or_default")]
    pub map_tz: TZ,

    // stats from the most recent map data query, as a diagnostic
    #[serde(deserialize_with = "ok_or_default")]
    pub last_map_query: Option<MapQueryStats>,
}

/// Diagnostic stats from a map data query, summed over the mounted databases.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MapQueryStats {
    pub n_points: usize,
    /// Number of line segments in the tileset. Not derivable from n_points:
    /// lines may be off, and out-of-bounds segments are skipped.
    pub n_segs: usize,
    /// Approximate bytes of the backend tile working set, the N-scaling
    /// memory cost of the query result. u64, not usize: this struct is
    /// deserialized on 32-bit wasm, where a usize slot truncates or fails
    /// past 4 GiB.
    pub tileset_bytes: u64,
    pub duration_ms: u64,
    /// True if any mounted database's result was decimated (every nth
    /// point in temporal mode, bucketed in spatial mode).
    pub decimated: bool,
    /// The per-mount memory backstop the query ran with, so a follow-up
    /// query (get_location_near) can reproduce the exact decimation.
    pub hard_cap: u64,
    /// True if the backend memory backstop bound any mount's result: a
    /// coarser sample in temporal mode, the oldest cells dropped in
    /// spatial mode. Never expected in normal use
    /// (doc/decimation/memory-limits.md); surfaced so the
    /// backstop can't bind silently.
    pub memory_capped: bool,
    /// Physical footprint of the app/server process after the update
    /// (dirty + compressed), the number the iOS per-process jetsam limit
    /// is enforced against. 0 if unavailable.
    pub footprint_bytes: u64,
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
    pub selected_points: Vec<((MountID, time::OffsetDateTime), LngLat)>,
    #[serde(deserialize_with = "ok_or_default")]
    pub copy_dest_db: MountID,

    #[serde(deserialize_with = "ok_or_default")]
    pub pin_import_default: Pin,

    #[serde(deserialize_with = "ok_or_default")]
    pub pin_settings: PinSettings,

    #[serde(deserialize_with = "ok_or_default")]
    pub notif_pref: NotificationPreference,

    // settings for the mounted databases, including the main database at idx 0
    #[serde(deserialize_with = "ok_or_default")]
    pub mounted_db_settings: BTreeMap<MountID, MountedDB>,
}

/// The page the frontend is on. Only variants that we care to persist between
/// launches are stored.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedRoute {
    Sense,
    Analyze,
    Places,
    Metrics,
    SettingsRoot,
    SettingsSubpage,
    #[default]
    Intro,
}

/// The settings page the frontend is on
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PersistedSettingsRoute {
    #[default]
    Root,
    General,
    MapSettings,
    Import,
    Export,
    Mounted,
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
    // the current pin in the PinEditor). Can be set independent of current_pin
    // when moving to the map, as current_pin is derived from it.
    #[serde(deserialize_with = "ok_or_default")]
    pub selected_pin_id: Option<i64>,
    #[serde(deserialize_with = "ok_or_default")]
    pub open_in_google_maps: bool,
    #[serde(deserialize_with = "ok_or_default")]
    pub popup_color: Option<String>, // color of popup background to display
    #[serde(deserialize_with = "ok_or_default")]
    pub timeline_config: TimelineConfig,
    // Step size and target ends for the time range stepper controls
    #[serde(deserialize_with = "ok_or_default")]
    pub time_step: TimeStep,
    #[serde(deserialize_with = "ok_or_default")]
    pub time_step_target: StepTarget,
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
            timeline_config: Default::default(),
            time_step: Default::default(),
            time_step_target: Default::default(),
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
