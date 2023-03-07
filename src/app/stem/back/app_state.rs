//! The state of the app's configuration

use std::sync::{Arc, Mutex};
// because we are using a standard Mutex, we cannot hold it across .await points

pub type AppState = Arc<AppStateContents>;

#[derive(Debug)]
pub struct AppStateContents {
    pub location_is_enabled: Mutex<bool>,
    pub distance_filter: Mutex<f32>,
}

pub trait AppStateExtentions {
    fn init_state() -> Arc<AppStateContents>;
}

impl AppStateExtentions for AppState {
    /// AppStateContents is wrapped in an Arc so it can be passed between
    /// threads. Actix already does this for web::Data<>, but we need to share
    /// it with code outside the webserver, so we put up with the overhead of
    /// having two Arcs.
    ///
    /// See https://actix.rs/docs/application/#state for more info.
    fn init_state() -> Arc<AppStateContents> {
        Arc::new(AppStateContents {
            // TODO: read from file or do default
            location_is_enabled: Mutex::new(false),
            distance_filter: Mutex::new(5.0),
        })
    }
}
