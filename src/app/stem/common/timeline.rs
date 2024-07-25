//! Timeline of movement and dwells

use serde::{Deserialize, Serialize};

use crate::{pin::Pin, LngLat, TimeRange};

pub type Timeline = Vec<Period>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Period {
    pub kind: PeriodKind,
    pub time: TimeRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeriodKind {
    Movement(Movement),
    Dwell(Dwell),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Movement {
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dwell {
    pub lnglat: LngLat,
    pub deviation: f64, // standard deviation of distances to lnglat
    pub detected_pin: Option<(Pin, f64)>, // detected pin and distance to lnglat
}
