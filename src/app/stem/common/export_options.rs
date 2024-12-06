//! Options for exporting logged data.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

pub const DEFAULT_MAX_POINTS: u64 = 10_000;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub view_bounded: bool,
    pub max_points: u64,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Scoria,
            view_bounded: true,
            max_points: DEFAULT_MAX_POINTS,
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum ExportFormat {
    #[strum(serialize = "Scoria")]
    Scoria,
    #[strum(serialize = "CSV")]
    CSV,
    #[strum(serialize = "GPX")]
    GPX,
    // GeoJson,
    // KML,
}
