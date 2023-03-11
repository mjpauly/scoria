//! The state of the app's configuration
//!
//! For unit testing we make it a thread_local. Wrapping in an extra Arc is
//! necessary since we can't pass references to thread_locals.

use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use sqlx::SqlitePool;

use crate::paths::Paths;

#[cfg(not(test))]
static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();
#[cfg(test)]
thread_local!(static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new());

#[derive(Debug)]
pub struct AppState {
    // App directory paths
    pub paths: Mutex<Paths>,

    // SqlitePool connection, currently shared between threads with Mutex
    pub db: SqlitePool,

    pub location_is_enabled: Mutex<bool>,
    pub distance_filter: Mutex<f32>,
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
        APP_STATE.with(|state| Self::do_global(state))
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
                paths: Mutex::new(paths),
                db,
                location_is_enabled: Mutex::new(true),
                distance_filter: Mutex::new(5.0),
            }))
            .expect("Could not initialize AppState");
    }
}
