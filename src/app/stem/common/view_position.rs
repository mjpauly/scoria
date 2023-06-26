//! A position from which the map is being viewed.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ViewPosition {
    pub lng: f64,
    pub lat: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}
