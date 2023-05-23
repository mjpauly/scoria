//! State that is mutually intelligible between the frontend and the
//! backend, and is saved between app launches
//!
//! We split this state in two to ensure we don't encounter race conditions when
//! the frontend and the backend both update the state at the same time. The
//! FrontState is driven by the frontend, and the BackState is driven by the
//! backend.

use serde::{Deserialize, Serialize};

use crate::{AutoConfig, Location, UserConfig};

/// Driven by frontend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FrontState {
    // Location configuration
    pub location_config: UserConfig,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: bool,
}

/// Driven by backend
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackState {
    pub auto_location_config: AutoConfig,

    pub last_location: Option<Location>,
    pub locations_past_hour: Option<i32>,
}
