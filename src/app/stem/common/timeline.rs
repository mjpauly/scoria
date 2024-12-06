//! Timeline of movement and dwells

use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::{pin::Pin, LngLat};

pub type Timeline = Vec<Period>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Period {
    pub kind: PeriodKind,
    pub start: Zoned,
    pub end: Zoned,
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

// ===== Config =====

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineConfig {
    pub long_dwell_width_secs: u64, // threshold duration for a dwell
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            long_dwell_width_secs: 120, // 2 minutes default
        }
    }
}
