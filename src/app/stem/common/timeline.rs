//! Timeline of movement and dwells

use serde::{Deserialize, Serialize};

use crate::{LngLat, TimeRange};

pub type Timeline = Vec<Period>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Period {
    Movement(Movement),
    Dwell(Dwell),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Movement {
    pub time: TimeRange,
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dwell {
    pub time: TimeRange,
    pub lnglat: LngLat,
}
