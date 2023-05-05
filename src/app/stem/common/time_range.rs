use serde::{Deserialize, Serialize};

/// A range of times.
/// Encoding as a struct helps ensure `start` and `end` are not accidentally
/// swapped.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: time::OffsetDateTime,
    pub end: time::OffsetDateTime,
}

impl TimeRange {
    pub fn contains(&self, timestamp: &time::OffsetDateTime) -> bool {
        &self.start <= timestamp && timestamp < &self.end
    }
}
