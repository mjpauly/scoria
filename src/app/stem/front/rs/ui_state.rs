//! Global state manager for the UI. Yewdux is used to provide access to the
//! global state (Store trait).
//!
//! Default values are None for Options and false for bools. LocationConfig has
//! its own Default implementation.

use yewdux::prelude::*;

use crate::websocket::{Callback, ToFront};
use common::{Location, LocationConfig};

#[derive(Debug, Clone, PartialEq, Default, Store)]
pub struct UIState {
    // Location state
    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,

    // Location configuration
    pub location_config: LocationConfig,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: bool,
}

impl UIState {
    pub fn get_update_callback() -> Callback {
        let dispatch = Dispatch::<UIState>::new();
        let callback = move |msg: &ToFront| match msg {
            ToFront::LocationConfig(val) => {
                dispatch.reduce_mut(|s| s.location_config = val.clone())
            }
            ToFront::LastLocation(val) => {
                dispatch.reduce_mut(|s| s.last_location = Some(val.clone()))
            }
            ToFront::LocationsPastHour(val) => {
                dispatch.reduce_mut(|s| s.locations_past_hour = Some(*val))
            }
            // This message handled by other callbacks, and not stored globally
            ToFront::LocationTimeRange(..) => (),
        };
        Box::new(callback)
    }
}
