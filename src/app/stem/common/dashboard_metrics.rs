use serde::{Deserialize, Serialize};

/// Metrics for the stats dashboard that are computed from the data visible on
/// the map.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardMetrics {
    pub total_distance: Option<f64>,
    pub start_time: Option<time::OffsetDateTime>,
    pub end_time: Option<time::OffsetDateTime>,
    pub duration: Option<time::Duration>,
    pub min_speed: Option<f64>,
    pub max_speed: Option<f64>,
    pub avg_speed: Option<f64>,
}
