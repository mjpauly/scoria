//! TimeRange could be pair of Timestamps. Just marks instants in time.
//!
//! TimeDeltaRange could be Spans for start/end offsets. A snap-to-day, and a
//! timezone name.
//!
//! Have ability to lookup/follow timezone in center of the map. Show tz when
//! selected.
//!
//! Ability to step by Span units, optionally with start/end tied together
//!
//! Start
//! End
//! +/- y m d H M S  [start, end, both]
//! All, week, day, now
//!
//! Timezone (localized, system / fixed) (just use setting?)

use jiff::{
    civil::{Date, Time},
    Span, Timestamp,
};
use serde::{Deserialize, Serialize};

use crate::state::ok_or_default;

/// A newtype which defines the default value for the TZ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TZ(pub String);

impl Default for TZ {
    fn default() -> Self {
        Self("UTC".into())
    }
}

impl std::ops::Deref for TZ {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// A range of times.
/// Encoding as a struct helps ensure `start` and `end` are not accidentally
/// swapped.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    #[serde(deserialize_with = "ok_or_default")]
    pub start: Timestamp,
    #[serde(deserialize_with = "ok_or_default")]
    pub end: Timestamp,
}

impl TimeRange {
    /// Slice semantics (start <= other < end)
    pub fn contains(&self, timestamp: &Timestamp) -> bool {
        &self.start <= timestamp && timestamp < &self.end
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        TimeDeltaRange::default()
            .to_time_range("UTC", Time::MIN)
            .unwrap()
    }
}

/// A range of times, defined by time deltas from now. (delta = then - now)
/// Used as the persistent data store for the map view
/// Negative offsets are backwars in time, positive are forward.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeDeltaRange {
    #[serde(deserialize_with = "ok_or_default")]
    pub start_offset: Span,
    #[serde(deserialize_with = "ok_or_default")]
    pub end_offset: Span,
    // whether to snap to the nearest day
    pub snap_start_to_day: bool,
    pub snap_end_to_day: bool,
}

impl TimeDeltaRange {
    pub fn to_time_range(
        &self,
        tz: &str,
        day_separation_time: Time,
    ) -> Result<TimeRange, jiff::Error> {
        let now = Timestamp::now().intz(tz)?;
        let mut start = &now + self.start_offset;
        let mut end = &now + self.end_offset;
        if self.snap_start_to_day {
            let delta = start.time() - day_separation_time;
            // need to provide the Zoned reference time for comparison
            start = if delta.compare((Span::new(), &start))?
                != std::cmp::Ordering::Less
            {
                // delta >= 0; past day separator
                &start - delta
            } else {
                // delta < 0; not yet at day separator
                &(&start - delta) - Span::new().days(1)
            }
        }
        if self.snap_end_to_day {
            let delta = end.time() - day_separation_time;
            end = if delta.compare((Span::new(), &end))?
                != std::cmp::Ordering::Less
            {
                // delta >= 0; past day separator
                &(&end - delta) + Span::new().days(1)
            } else {
                // delta < 0; not yet at day separator
                &end - delta
            }
        }
        Ok(TimeRange {
            start: start.timestamp(),
            end: end.timestamp(),
        })
    }

    pub fn today() -> Self {
        Self {
            start_offset: Span::new(),
            end_offset: Span::new(),
            snap_start_to_day: true,
            snap_end_to_day: true,
        }
    }

    pub fn past_24h() -> Self {
        Self {
            start_offset: -Span::new().days(1),
            end_offset: Span::new(),
            snap_start_to_day: false,
            snap_end_to_day: false,
        }
    }

    pub fn week() -> Self {
        Self {
            start_offset: -Span::new().weeks(1),
            end_offset: Span::new(),
            snap_start_to_day: true,
            snap_end_to_day: true,
        }
    }

    /// Show a particular date, snapped to the day
    pub fn date(date: Date, tz: &str) -> Result<Self, jiff::Error> {
        let now = Timestamp::now().intz(tz)?;
        let delta = now.until(&date.at(12, 0, 0, 0).intz(tz)?)?; // noon
        Ok(Self {
            start_offset: delta,
            end_offset: delta,
            snap_start_to_day: true,
            snap_end_to_day: true,
        })
    }

    pub fn range(tr: TimeRange) -> Result<Self, jiff::Error> {
        let now = Timestamp::now();
        let start_delta = now.until(tr.start)?;
        let end_delta = now.until(tr.end)?;
        Ok(Self {
            start_offset: start_delta,
            end_offset: end_delta,
            snap_start_to_day: true,
            snap_end_to_day: true,
        })
    }
}

impl Default for TimeDeltaRange {
    /// Time range to use if it can't be parsed from file. This is a backup in
    /// case deserialization doesn't work. On first install the frontend should
    /// be what initializes the time_range.
    fn default() -> Self {
        Self::today()
    }
}
