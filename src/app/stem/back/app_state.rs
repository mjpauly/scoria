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

use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use sqlx::SqlitePool;

use crate::common::LocationAccuracyMode;
use crate::paths::Paths;
use crate::ws_session;

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

    pub location_is_enabled: Mutex<bool>,
    pub distance_filter: Mutex<f32>,
    pub significant_changes: Mutex<bool>,
    pub location_accuracy_mode: Mutex<LocationAccuracyMode>,
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
        // TODO: pull saved settings from file
        (*state)
            .set(Arc::new(AppState {
                paths,
                db,
                ws_addr: Mutex::new(None),
                location_is_enabled: Mutex::new(false),
                distance_filter: Mutex::new(5.0),
                significant_changes: Mutex::new(false),
                location_accuracy_mode: Mutex::new(LocationAccuracyMode::Best),
            }))
            .expect("Could not initialize AppState");
    }
}
