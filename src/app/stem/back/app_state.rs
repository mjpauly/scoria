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
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::core::log_with_dir;
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
}

/// State that is persisted across app launches. Consists of two components that
/// are driven by the frontend and the backend respectively
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(default)]
pub struct PersistentState {
    // We let the frontend initialize its state, so it starts out as None in the
    // backend until it is sent. This is needed since the backend can't derive
    // certain values, such as a time range that uses the local time zone
    // offset, due to a security flaw.
    pub front: Option<FrontState>,
    pub back: BackState,
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
        let persistent = if let Ok(input) = fs::read_to_string(state_file) {
            match serde_json::from_str(&input) {
                Ok(parsed) => {
                    println!("Successfully loaded app state: {:?}", parsed);
                    parsed
                }
                Err(e) => {
                    let msg =
                        &format!("Failed to parse state due to error: {}", e);
                    println!("{}", msg);
                    log_with_dir(msg, &paths.documents_dir);
                    PersistentState::default()
                }
            }
        } else {
            PersistentState::default()
        };
        (*state)
            .set(Arc::new(AppState {
                paths,
                db,
                ws_addr: Mutex::new(None),
                server_handle: tokio::sync::Mutex::new(None),
                persistent: Mutex::new(persistent),
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
    use common::{
        state::MapState, AutoConfig, LocationAccuracyMode, LocationMode,
        StandardLocationConfig, TimeRange,
    };

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
                use_epsln_tile_server: false,
                map: MapState {
                    time_range: TimeRange {
                        start: time::OffsetDateTime::now_utc(),
                        end: time::OffsetDateTime::now_utc(),
                    },
                    style: Default::default(),
                    filters: vec![],
                    view_pos: Default::default(),
                },
                route: Default::default(),
            }),
            back: BackState {
                last_location: None,
                locations_past_hour: None,
                auto_location_config: AutoConfig::default(),
            },
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
}
