//! Global state manager for the UI

use yewdux::prelude::*;

use crate::common::{Location, LocationAccuracyMode};
use crate::websocket::{Callback, ToFront};

#[derive(Debug, Clone, PartialEq, Store)]
pub struct UIState {
    // Location state
    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,

    // Location configuration
    pub location_is_enabled: bool,
    pub distance_filter: f32,
    pub significant_changes: bool,
    pub location_accuracy_mode: LocationAccuracyMode,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            last_location: None,
            locations_past_hour: None,

            location_is_enabled: false,
            distance_filter: 5.0,
            significant_changes: false,
            location_accuracy_mode: LocationAccuracyMode::Best,

            use_epsln_tile_server: false,
        }
    }
}

impl UIState {
    pub fn get_update_callback() -> Callback {
        let dispatch = Dispatch::<UIState>::new();
        let callback = move |msg: &ToFront| match msg {
            ToFront::LastLocation(val) => {
                dispatch.reduce_mut(|s| s.last_location = Some(val.clone()))
            }
            ToFront::LocationsPastHour(val) => {
                dispatch.reduce_mut(|s| s.locations_past_hour = Some(*val))
            }
            ToFront::LocationEnabled(val) => {
                dispatch.reduce_mut(|s| s.location_is_enabled = *val)
            }
            ToFront::DistFilt(val) => {
                dispatch.reduce_mut(|s| s.distance_filter = *val)
            }
            ToFront::SignificantChanges(val) => {
                dispatch.reduce_mut(|s| s.significant_changes = *val)
            }
            ToFront::LocationAccuracyMode(val) => {
                dispatch.reduce_mut(|s| s.location_accuracy_mode = *val)
            }
            // This message handled by other callbacks, and not stored globally
            ToFront::LocationTimeRange(..) => (),
        };
        Box::new(callback)
    }
}
