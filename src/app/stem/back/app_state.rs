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

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

use actix_web::dev::ServerHandle;
use common::state::{ok_or_default, MapState};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::core::{log_with_dir, timestamp};
use crate::geojson::Geojson;
use crate::paths::{get_library_dir, Paths};
use crate::ws_session;
use common::{BackState, FrontState};

/// File where persistent state is stored (joined to library_dir)
static STATE_FNAME: &str = "persistent_state.json";

#[cfg(not(test))]
static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();
#[cfg(test)]
thread_local!(static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new());

#[derive(Debug)]
pub struct AppState {
    // App directory paths, set during startup so not mutable
    pub paths: Paths,

    // SqlitePool connection, sharable between threads and clonable
    pub db: SqlitePool,

    // Address of the websocket actor so we can send messages to it
    pub ws_addr: Mutex<Option<actix::Addr<ws_session::WsSession>>>,
    // Need an async-aware mutex if we are to await server shutdown with it held
    pub server_handle: tokio::sync::Mutex<Option<ServerHandle>>,

    // State persisted between app launches. It is almost exactly the same as
    // the UI state.
    pub persistent: Mutex<PersistentState>,

    pub swift_messages: Mutex<SwiftMessages>,

    pub map_data: Mutex<MapData>,
}

/// Data to to shown on the map in the analyze tab, and helpers for calculating
/// it.
#[derive(Debug, Default)]
pub struct MapData {
    // data to plot (gets stringified in the actix route)
    pub points_geojson: Geojson,
    pub lines_geojson: Geojson,
    // previous map state to determine if an update is needed
    pub prev_map_state: Option<MapState>,
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

/// Temporary data to communicate to Swift
#[derive(Debug, Default)]
pub struct SwiftMessages {
    pub should_request_when_in_use_authorization: bool,
    // tell swift to export the SQLite log in a share sheet
    pub should_export_sqlite_log: bool,
    // tell swift to import the SQLite log
    pub should_import_sqlite_log: bool,
}

impl AppState {
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
    pub fn init(paths: Paths, db: SqlitePool) {
        Self::do_init(&APP_STATE, paths, db);
    }

    /// Initialize the AppState
    #[cfg(test)]
    pub fn init(paths: Paths, db: SqlitePool) {
        APP_STATE.with(|state| {
            Self::do_init(state, paths, db);
        });
    }

    /// Actual init implementation shared between both test and non-test cases
    fn do_init(state: &OnceCell<Arc<AppState>>, paths: Paths, db: SqlitePool) {
        let state_file = paths.library_dir.join(STATE_FNAME);
        let persistent = match fs::read_to_string(state_file) {
            Ok(input) => match serde_json::from_str(&input) {
                Ok(parsed) => {
                    let msg = &format!(
                        "DEBUG {} Successfully loaded app state:\n {:?}\n
                         File contents were \"{}\"",
                        timestamp(),
                        parsed,
                        input
                    );
                    println!("{}", msg);
                    log_with_dir(msg, &paths.documents_dir);
                    parsed
                }
                Err(e) => {
                    let msg = &format!(
                        "DEBUG {} Failed to parse state due to error: {}\
                        File contents were \"{}\"",
                        timestamp(),
                        e,
                        input
                    );
                    println!("{}", msg);
                    log_with_dir(msg, &paths.documents_dir);
                    PersistentState::default()
                }
            },
            Err(e) => {
                let msg = &format!(
                    "DEBUG {} Failed to read state file due to error: {}",
                    timestamp(),
                    e
                );
                println!("{}", msg);
                log_with_dir(msg, &paths.documents_dir);
                PersistentState::default()
            }
        };
        (*state)
            .set(Arc::new(AppState {
                paths,
                db,
                ws_addr: Mutex::new(None),
                server_handle: tokio::sync::Mutex::new(None),
                persistent: Mutex::new(persistent),
                swift_messages: Mutex::new(Default::default()),
                map_data: Mutex::new(Default::default()),
            }))
            .expect("Could not initialize AppState");
    }

    pub fn save_to_file() {
        let state_file = get_library_dir().join(STATE_FNAME);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // delete previous contents that are longer
            .open(state_file)
            .unwrap();
        let state_str =
            serde_json::to_string(&*Self::global().persistent.lock().unwrap())
                .unwrap();
        file.write_all(state_str.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::init;
    use crate::local::local_fs_setup;
    use common::{LocationAccuracyMode, LocationMode, StandardLocationConfig};

    use super::{
        fs, AppState, BackState, FrontState, OpenOptions, PersistentState,
        Write, STATE_FNAME,
    };

    #[tokio::test]
    async fn state_serialization_works() {
        let dir = "state_serialization_works/";
        let paths = local_fs_setup(dir);
        let state_file = paths.library_dir.clone().join(STATE_FNAME);
        init(paths).await;

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

        init(paths).await;

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

        let contents = r##"{"front":{"last_viewed_intro_version":0,"route":"DefinitelyNotARoute","settings_route":"Root","use_epsln_tile_server":true},"back":{"locations_past_hour":2,"cmap_params":{"cmap":"NotARealCmap"}}}"##;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(state_file)
            .unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        init(paths).await;

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
        assert!(parsed.front.as_ref().unwrap().use_epsln_tile_server);
        assert_eq!(parsed.back.locations_past_hour, Some(2));

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
}
