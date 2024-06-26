use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A range of times.
/// Encoding as a struct helps ensure `start` and `end` are not accidentally
/// swapped.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl TimeRange {
    pub fn contains(&self, timestamp: &OffsetDateTime) -> bool {
        &self.start <= timestamp && timestamp < &self.end
    }
}

impl From<&TimeDeltaRange> for TimeRange {
    fn from(td_range: &TimeDeltaRange) -> Self {
        let mut now = OffsetDateTime::now_utc();
        if let Some(offset) = td_range.offset {
            now = now.to_offset(offset);
        }
        let mut start = now + td_range.start_offset;
        let mut end = now + td_range.end_offset;
        if td_range.snap_start_to_day {
            start = start.replace_time(time::Time::MIDNIGHT);
        }
        if td_range.snap_end_to_day {
            end = end.replace_time(time::Time::from_hms(23, 59, 59).unwrap());
        }
        Self { start, end }
    }
}

/// A range of times, defined by time deltas from now. (delta = then - now)
/// Used as the persistent data store for the map view
/// Negative offsets are backwars in time, positive are forward.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeDeltaRange {
    pub start_offset: time::Duration,
    pub end_offset: time::Duration,
    // whether to snap to the start/end of the day
    pub snap_start_to_day: bool,
    pub snap_end_to_day: bool,
    // an optional utc offset so snap-to-day works with local time
    pub offset: Option<time::UtcOffset>,
}

impl Default for TimeDeltaRange {
    /// Time range to use if it can't be parsed from file. Does not depend on
    /// the timezone offset, so it is safe to be run by the backend. This is a
    /// backup in case deserialization doesn't work. On first install the
    /// frontend should be what initializes the time_range.
    fn default() -> Self {
        Self {
            start_offset: -time::Duration::DAY,
            end_offset: time::Duration::DAY,
            snap_start_to_day: false,
            snap_end_to_day: false,
            offset: None,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        (&TimeDeltaRange::default()).into()
    }
}
