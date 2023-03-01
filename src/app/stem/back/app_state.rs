//! The state of the app's configuration

use std::sync::{Arc, Mutex};
// because we are using a standard Mutex, we cannot hold it across .await points

#[derive(Debug)]
pub struct AppStateContents {
    location_is_enabled: bool,
    distance_filter: f32,
}

pub type AppState = Arc<Mutex<AppState>>;

pub fn get_app_state() -> AppState {
    Arc::new(Mutex::new(AppState {
        // TODO: read from file or do default
        location_is_enabled: false,
        distance_filter: 5,
    }))
}

pub fn get_location_is_enabled(state: AppState) -> bool {
    state.lock().unwrap().location_is_enabled
}

pub fn set_location_is_enabled(state: AppState, enabled: bool) {
    state.lock().unwrap().location_is_enabled = enabled;
}
