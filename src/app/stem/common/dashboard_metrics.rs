use serde::{Deserialize, Serialize};

/// Metrics for the stats dashboard that are computed from the data visible on
/// the map.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardMetrics {
    pub count: u64,
    pub total_distance: f64, // (skips dwell dithering)
    pub start_time: Option<time::OffsetDateTime>,
    pub end_time: Option<time::OffsetDateTime>,
    pub dwell_time: time::Duration,
    pub movement_time: time::Duration,
    pub min_speed: Option<f64>,
    pub max_speed: Option<f64>,
    pub avg_speed: Option<f64>,
}

impl DashboardMetrics {
    /// Merge the stats from two sets of metrics, such as for two separate
    /// segments.
    pub fn merge_other(&self, other: &Self) -> Self {
        // compute these first so they can be used for the avg_speed
        let total_distance = self.total_distance + other.total_distance;
        let movement_time = self.movement_time + other.movement_time;
        Self {
            count: self.count + other.count,
            total_distance,
            start_time: merge_option(
                self.start_time,
                other.start_time,
                |a, b| a.min(b),
            ),
            end_time: merge_option(self.end_time, other.end_time, |a, b| {
                a.max(b)
            }),
            dwell_time: self.dwell_time + other.dwell_time,
            movement_time,
            min_speed: merge_option(self.min_speed, other.min_speed, |a, b| {
                a.min(b)
            }),
            max_speed: merge_option(self.max_speed, other.max_speed, |a, b| {
                a.max(b)
            }),
            avg_speed: (movement_time > time::Duration::ZERO)
                .then(|| total_distance / movement_time.as_seconds_f64()),
        }
    }
}

/// Take two options and merge their Some variants with the provided Fn, or
/// return only the option with Some.
fn merge_option<T>(
    a: Option<T>,
    b: Option<T>,
    f: impl Fn(T, T) -> T,
) -> Option<T> {
    match (a, b) {
        (Some(a_val), Some(b_val)) => Some(f(a_val, b_val)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}
