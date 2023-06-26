//! Types common to the front and back ends
//!
//! Messages are serialized with bincode.

use serde::{Deserialize, Serialize};

pub mod cmaps;
pub mod filters;
pub mod float;
pub mod location;
pub mod location_config;
pub mod map_style;
pub mod state;
pub mod time_range;
pub mod units;
pub mod view_position;
pub mod ws_messages;

pub use location::Location;
pub use location_config::{
    AllLocationConfig, AutoConfig, LocationAccuracyMode, LocationMode,
    OSLocationMode, StandardLocationConfig, UserConfig,
};
pub use state::{BackState, FrontState};
pub use time_range::TimeRange;
pub use ws_messages::{ToBack, ToFront};

#[derive(PartialEq, Debug, Default, Clone, Serialize, Deserialize)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}
