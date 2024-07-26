//! Types common to the front and back ends
//!
//! Messages are serialized with bincode.

pub mod cmaps;
pub mod dashboard_metrics;
pub mod export_options;
pub mod filters;
pub mod float;
pub mod lnglat;
pub mod location;
pub mod location_config;
pub mod map_style;
pub mod pin;
pub mod plot_data;
pub mod popups;
pub mod state;
pub mod time_range;
pub mod timeline;
pub mod units;
pub mod validation;
pub mod view_position;
pub mod ws_messages;

pub use lnglat::LngLat;
pub use location::Location;
pub use location_config::{
    AllLocationConfig, AutoConfig, LocationAccuracyMode, LocationMode,
    OSLocationMode, StandardLocationConfig, UserConfig,
};
pub use state::{BackState, FrontState};
pub use time_range::TimeRange;
pub use ws_messages::{ToBack, ToFront};
