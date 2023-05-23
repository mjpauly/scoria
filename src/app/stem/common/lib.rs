//! Types common to the front and back ends
//!
//! Messages are serialized with bincode.

pub mod location;
pub mod location_config;
pub mod state;
pub mod time_range;
pub mod ws_messages;

pub use location::Location;
pub use location_config::{
    AllLocationConfig, AutoConfig, LocationAccuracyMode, LocationMode,
    OSLocationMode, StandardLocationConfig, UserConfig,
};
pub use state::{BackState, FrontState};
pub use time_range::TimeRange;
pub use ws_messages::{ToBack, ToFront};
