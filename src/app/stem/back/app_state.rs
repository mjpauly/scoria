//! The state of the app's configuration
//!
//! Some fields are not wrapped in a Mutex. This is if the data is set on
//! initialization and never changes during runtime, or if the datatype is
//! already safe to send across threads (Sync + Send).
//!
//! Some data is initialized to a default value at startup, but is wrapped in a
//! Mutex so it can be changed. Some data might not always be present, so is
//! also wrapped in an Option<>.
//!
//! For unit testing we make it a thread_local. Wrapping in an extra Arc is
//! necessary since we can't pass references to thread_locals.

use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

use actix_web::dev::ServerHandle;
use anyhow::Context;
use common::mounted::MountID;
use common::state::{ok_or_default, DerivedState, MapState, PendingEvents};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::error;

use crate::logs::LogErrorAndContinue;
use crate::paths::{get_library_dir, Paths};
use crate::ws_session::{self, send_derived_state_to_front};
use common::{BackState, FrontState};

/// File where persistent state is stored (joined to library_dir)
static STATE_FNAME: &str = "persistent_state.json";
static STATE_TMP_FNAME: &str = "persistent_state.json.tmp";

#[cfg(not(test))]
static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();
#[cfg(test)]
thread_local!(static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new());

#[derive(Debug)]
pub struct AppState {
    // App directory paths, set during startup so not mutable
    pub paths: Paths,

    // Mounted databases that can be opened successfully, including the main
    // database at index 0. Errors for dbs that are on disk, but not openable,
    // in common::state::DerivedState.mounted_dbs_on_disk. BTreeMap is used
    // since it's ordered, which is nice for displaying, and has fast access to
    // the first element.
    pub dbs: Mutex<BTreeMap<MountID, SqlitePool>>,
    // locations logged while the main database was still opening
    pub pending_locations: Mutex<Vec<crate::database::OSLocationData>>,

    // Address of the websocket actor so we can send messages to it
    pub ws_addr: Mutex<Option<actix::Addr<ws_session::WsSession>>>,
    // Need an async-aware mutex if we are to await server shutdown with it held
    pub server_handle: tokio::sync::Mutex<Option<ServerHandle>>,

    // State persisted between app launches. It is almost exactly the same as
    // the UI state.
    pub persistent: Mutex<PersistentState>,
    // Non-persistent backend state that is derived from other sources
    pub derived: Mutex<DerivedState>,

    pub wrapper_messages: Mutex<WrapperMessages>,

    pub map_data: MapData,

    // events that accumulate until they are sent to the UI and cleared
    pub pending_events: Mutex<Option<PendingEvents>>,
}

/// Data to to shown on the map in the analyze tab, and helpers for calculating
/// it.
#[derive(Debug)]
pub struct MapData {
    // locks to limit only one task to do update computations and one task to
    // wait for the previous to finish
    pub map_data_wait_lock: tokio::sync::Mutex<()>,
    pub map_data_update_lock: tokio::sync::Mutex<()>,
    // per-mount working set for the vector tile route (mvt.rs), in z0
    // web-mercator coordinates. RwLock + Arc so concurrent tile requests
    // can slice in parallel without holding the map lock
    pub mount_tilesets: AsyncRwLock<
        BTreeMap<MountID, std::sync::Arc<crate::map::mvt::TileSet>>,
    >,
    // previous map state to determine if an update is needed
    pub prev_map_state: tokio::sync::Mutex<Option<MapState>>,
    // previous map state when timeline was lasts updated
    pub prev_map_state_dashboard: tokio::sync::Mutex<Option<MapState>>,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            map_data_wait_lock: tokio::sync::Mutex::new(()),
            map_data_update_lock: tokio::sync::Mutex::new(()),
            mount_tilesets: AsyncRwLock::new(BTreeMap::new()),
            prev_map_state: tokio::sync::Mutex::new(None),
            prev_map_state_dashboard: tokio::sync::Mutex::new(None),
        }
    }
}

/// State that is persisted across app launches. Consists of two components that
/// are driven by the frontend and the backend respectively
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(default)]
pub struct PersistentState {
    // We let the frontend initialize its state, so it starts out as None in the
    // backend until it is sent. This is needed since the backend can't derive
    // certain values, such as a time range that uses the local time zone
    // offset, due to troubles with getting the time zone (or rather just
    // not feeling confident that it's safe enough to do so).
    #[serde(deserialize_with = "ok_or_default")]
    pub front: Option<FrontState>,
    #[serde(deserialize_with = "ok_or_default")]
    pub back: BackState,
}

/// Modify the BackState using the provided closure. Can be used to retrieve a
/// value from the BackState, or the whole back state with a clone(). Since
/// we're holding a lock, don't block in the body of the closure.
pub fn set_back_state<T>(f: impl FnOnce(&mut BackState) -> T) -> T {
    f(&mut AppState::global().persistent.lock().unwrap().back)
}

/// Retrive values from the BackState. Cannot modify the BackState.
pub fn get_back_state<T>(f: impl FnOnce(&BackState) -> T) -> T {
    f(&AppState::global().persistent.lock().unwrap().back)
}

/// Retrive values from the FrontState, which may or may not be initialized.
/// Cannot modify the FrontState.
pub fn get_front_state<T>(f: impl FnOnce(&FrontState) -> T) -> Option<T> {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front
        .as_ref()
        .map(f)
}

pub fn set_derived_state<T>(f: impl FnOnce(&mut DerivedState) -> T) -> T {
    let res = f(&mut AppState::global().derived.lock().unwrap());
    send_derived_state_to_front();
    res
}
pub fn get_derived_state<T>(f: impl FnOnce(&DerivedState) -> T) -> T {
    f(&AppState::global().derived.lock().unwrap())
}

/// Temporary data to communicate to Swift
#[derive(Debug, Default)]
pub struct WrapperMessages {
    pub should_request_when_in_use_authorization: bool,
    pub should_go_to_location_settings: bool,
    // tell swift to export the SQLite log in a share sheet
    pub should_export_sqlite_log: bool,
    // tell swift to import the SQLite log
    pub should_import_sqlite_log: bool,
    pub should_export_track: bool,
    pub should_import_places_geojson: bool,
    pub should_mount_db: bool,
    pub should_export_image: bool,
}

impl AppState {
    /// Whether the app state been initialized already. Initialization can only
    /// happen once.
    #[cfg(not(test))]
    pub fn is_initialized() -> bool {
        APP_STATE.get().is_some()
    }

    #[cfg(test)]
    pub fn is_initialized() -> bool {
        APP_STATE.with(|s| s.get().is_some())
    }

    /// Get the global AppState instance
    #[cfg(not(test))]
    pub fn global() -> Arc<AppState> {
        Self::do_global(&APP_STATE)
    }

    /// Get the global AppState instance
    #[cfg(test)]
    pub fn global() -> Arc<AppState> {
        APP_STATE.with(Self::do_global)
    }

    /// Actual global implementation shared between both test and non-test cases
    fn do_global(state: &OnceCell<Arc<AppState>>) -> Arc<AppState> {
        state.get().expect("AppState not initialized").clone()
    }

    /// Initialize the AppState
    #[cfg(not(test))]
    pub fn init(paths: Paths, app_version: String) {
        Self::do_init(&APP_STATE, paths, app_version);
    }

    /// Initialize the AppState
    #[cfg(test)]
    pub fn init(paths: Paths, app_version: String) {
        APP_STATE.with(|state| {
            Self::do_init(state, paths, app_version);
        });
    }

    /// Actual init implementation shared between both test and non-test cases
    fn do_init(
        state: &OnceCell<Arc<AppState>>,
        paths: Paths,
        app_version: String,
    ) {
        let state_file = paths.library_dir.join(STATE_FNAME);
        let mut persistent = match fs::read_to_string(state_file) {
            Ok(input) => match serde_json::from_str(&input) {
                Ok(parsed) => {
                    tracing::debug!(
                        "Successfully loaded app state:\n{parsed:?}"
                    );
                    parsed
                }
                Err(e) => {
                    // don't log the whole file: it ends up in the log, which
                    // the problem report attaches, so keep it bounded
                    error!(
                        "Failed to parse state ({} bytes): {e}. Near: {}",
                        input.len(),
                        error_context(&input, e.line(), e.column()),
                    );
                    PersistentState::default()
                }
            },
            Err(e) => {
                match e.kind() {
                    // if the file isn't found, assume it's a fresh install and
                    // don't log the error
                    std::io::ErrorKind::NotFound => {}
                    _ => {
                        error!("Failed to read state file. Err: {e}",);
                    }
                }
                PersistentState::default()
            }
        };
        persistent.back.app_version = app_version; // update app version
        (*state)
            .set(Arc::new(AppState {
                paths,
                dbs: Mutex::new(BTreeMap::new()),
                ws_addr: Mutex::new(None),
                server_handle: tokio::sync::Mutex::new(None),
                persistent: Mutex::new(persistent),
                derived: Mutex::new(Default::default()),
                wrapper_messages: Mutex::new(Default::default()),
                pending_locations: Mutex::new(Vec::new()),
                map_data: Default::default(),
                pending_events: Mutex::new(Default::default()),
            }))
            .expect("Could not initialize AppState");
    }

    /// Save the app state to the state file. Since things can continue to
    /// function with the persistent state, we just log errors and return early
    /// if they occur.
    pub fn save_to_file() {
        let state_file = get_library_dir().join(STATE_FNAME);
        // write to a sibling then rename, so a kill mid-write can't leave a
        // truncated state file
        let tmp_file = get_library_dir().join(STATE_TMP_FNAME);
        let mut file = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // delete previous contents that are longer
            .open(&tmp_file)
        {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to open state file for writing: {e}.");
                return;
            }
        };
        let state_str = match serde_json::to_string(
            &*Self::global().persistent.lock().unwrap(),
        ) {
            Ok(state_str) => state_str,
            Err(e) => {
                error!("Failed to create persistent state string: {e}.");
                return;
            }
        };
        if let Err(e) = file.write_all(state_str.as_bytes()) {
            error!("Failed to write persistent state: {e}.");
            return;
        }
        drop(file);
        fs::rename(tmp_file, state_file)
            .context("renaming persistent state file")
            .log_error_and_continue();
    }
}

/// A window of `input` around the 1-based line and column of a parse error.
fn error_context(input: &str, line: usize, column: usize) -> String {
    const RADIUS: usize = 200;
    let line_start = input
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(|l| l.len())
        .sum::<usize>();
    let pos = (line_start + column.saturating_sub(1)).min(input.len());
    let mut start = pos.saturating_sub(RADIUS);
    let mut end = (pos + RADIUS).min(input.len());
    while !input.is_char_boundary(start) {
        start -= 1;
    }
    while !input.is_char_boundary(end) {
        end += 1;
    }
    format!("{:?}", &input[start..end])
}

#[cfg(test)]
mod tests {
    use crate::init;
    use crate::local::local_fs_setup;
    use common::{LocationAccuracyMode, LocationMode, StandardLocationConfig};
    use pretty_assertions::assert_eq;

    use super::{
        fs, AppState, BackState, FrontState, OpenOptions, PersistentState,
        Write, STATE_FNAME,
    };

    #[tokio::test]
    async fn state_serialization_works() {
        let dir = "state_serialization_works/";
        let paths = local_fs_setup(dir);
        let state_file = paths.library_dir.clone().join(STATE_FNAME);
        init(paths, String::default()).await;

        // save state to file
        AppState::save_to_file();

        let serialized = fs::read_to_string(state_file).unwrap();
        let parsed: PersistentState =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(PersistentState::default(), parsed);
    }

    #[tokio::test]
    async fn state_deserialization_works() {
        let dir = "state_serde_works/";
        let paths = local_fs_setup(dir);
        let state_file = paths.library_dir.clone().join(STATE_FNAME);

        let state = PersistentState {
            front: Some(FrontState {
                location_config: common::UserConfig {
                    enabled: true,
                    mode: LocationMode::Standard,
                    standard_config: StandardLocationConfig {
                        accuracy_mode: LocationAccuracyMode::TenMeters,
                        distance_filter: 4.0,
                    },
                },
                ..Default::default()
            }),
            back: BackState::default(),
        };
        let contents = serde_json::to_string(&state).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(state_file)
            .unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        init(paths, String::default()).await;

        let parsed = (*AppState::global().persistent.lock().unwrap()).clone();
        assert_eq!(state, parsed);
    }

    /// Test that invalid fields in the persisted state file are deserialized as
    /// their default. This is important for cases when, for example, the name
    /// of an enum variant changes. Without the attribute
    /// `#[serde(deserialize_with = "ok_or_default")]`, the entire
    /// deserialization will fail.
    #[tokio::test]
    async fn state_deserialization_uses_default_for_invalid_fields() {
        let dir = "state_serde_deserialize_is_robust/";
        let paths = local_fs_setup(dir);
        let state_file = paths.library_dir.clone().join(STATE_FNAME);

        let contents = r##"{"front":{"last_viewed_intro_version":0,"route":"DefinitelyNotARoute","settings_route":"Root","use_scoria_tile_server":true},"back":{"locations_past_minute":2,"cmap_params":{"cmap":"NotARealCmap"}}}"##;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(state_file)
            .unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        init(paths, String::default()).await;

        let parsed = (*AppState::global().persistent.lock().unwrap()).clone();

        // can't compare the whole structs since they contain times that would
        // differ, so we just check the data we expect to have been correctly
        // persisted, and the state that should have been initialized from
        // default
        assert_eq!(parsed.front.as_ref().unwrap().last_viewed_intro_version, 0);
        assert_eq!(
            parsed.front.as_ref().unwrap().settings_route,
            common::state::PersistedSettingsRoute::Root
        );
        assert_eq!(parsed.back.locations_past_minute, Some(2));

        // values that should be the default
        assert_eq!(
            parsed.front.as_ref().unwrap().route,
            common::state::PersistedRoute::default()
        );
        assert_eq!(
            parsed.front.as_ref().unwrap().location_config,
            common::location_config::UserConfig::default()
        );
        assert_eq!(
            parsed.front.as_ref().unwrap().unit_pref,
            common::units::UnitPreference::default()
        );
        assert_eq!(
            parsed.back.cmap_params.cmap,
            common::cmaps::Cmap::default()
        );
    }

    /// The parse error context stays small and lands on the error position.
    #[test]
    fn error_context_window() {
        let input = "{\"a\":1,\n\"b\":".to_string() + &"x".repeat(1000);
        let e = serde_json::from_str::<serde_json::Value>(&input).unwrap_err();
        let ctx = super::error_context(&input, e.line(), e.column());
        assert!(ctx.contains("\\\"b\\\":x"), "{ctx}");
        assert!(ctx.len() < 450, "{ctx}");
        // out-of-range positions must not panic
        super::error_context("", 5, 5);
        super::error_context("héllo", 1, 3);
    }

    /// An unexpected enum value can cause "trailing character" errors when
    /// parsing, unless annotated with `ok_or_default` for enum fields.
    ///
    /// ```
    /// #[serde(deserialize_with = "ok_or_default")]
    /// pub colored_datastream: ColoredDataStream,
    /// ```
    #[tokio::test]
    async fn state_failure_case() {
        let s = r##"{"front":{"map":{"style":{"colored_datastream":"Unexpected","show_colorbar":false}}}}"##;
        let p: PersistentState = serde_json::from_str(s).unwrap();
        assert!(!p.front.unwrap().map.style.show_colorbar);
    }

    /// Check that we can deserialize something that is completely the wrong
    /// type, and get the default value out.
    #[tokio::test]
    async fn span_failure_case() {
        let s = r##"{"front":{"map":{"time_delta_range":{"start_offset":[0,0],"end_offset":[0,0],"snap_start_to_day":true,"snap_end_to_day":true,"offset":[-8,0,0]},"time_range":{"start":[2024,309,0,0,0,0,-8,0,0],"end":[2024,309,23,59,59,0,-8,0,0]}}}}"##;
        let _: PersistentState = serde_json::from_str(s).unwrap();
    }
}
