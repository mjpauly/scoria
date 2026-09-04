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

    /// Step start and/or end by the given calendar span, applied in the
    /// given timezone so day and larger units follow the wall clock across
    /// DST transitions. Returns None if the step would put start after end.
    pub fn stepped(
        &self,
        span: Span,
        target: StepTarget,
        tz: &str,
    ) -> Result<Option<Self>, jiff::Error> {
        let mut new = *self;
        if target.moves_start() {
            new.start = self.start.intz(tz)?.checked_add(span)?.timestamp();
        }
        if target.moves_end() {
            new.end = self.end.intz(tz)?.checked_add(span)?.timestamp();
        }
        Ok((new.start <= new.end).then_some(new))
    }
}

/// Preset step sizes for the time range stepper controls.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum TimeStep {
    Sec1,
    Sec5,
    Min1,
    Min5,
    Hour1,
    #[default]
    Day1,
    Day7,
    Month1,
    Year1,
}

impl TimeStep {
    pub const ALL: [Self; 9] = [
        Self::Sec1,
        Self::Sec5,
        Self::Min1,
        Self::Min5,
        Self::Hour1,
        Self::Day1,
        Self::Day7,
        Self::Month1,
        Self::Year1,
    ];

    pub fn value(&self) -> i64 {
        match self {
            Self::Sec1 | Self::Min1 | Self::Hour1 | Self::Day1 => 1,
            Self::Sec5 | Self::Min5 => 5,
            Self::Day7 => 7,
            Self::Month1 | Self::Year1 => 1,
        }
    }

    pub fn unit(&self) -> &'static str {
        match self {
            Self::Sec1 | Self::Sec5 => "s",
            Self::Min1 | Self::Min5 => "min",
            Self::Hour1 => "h",
            Self::Day1 | Self::Day7 => "d",
            Self::Month1 => "mo",
            Self::Year1 => "y",
        }
    }

    /// The step's span, scaled by `n` (which may be negative).
    pub fn span(&self, n: i64) -> Span {
        let amount = self.value() * n;
        match self {
            Self::Sec1 | Self::Sec5 => Span::new().seconds(amount),
            Self::Min1 | Self::Min5 => Span::new().minutes(amount),
            Self::Hour1 => Span::new().hours(amount),
            Self::Day1 | Self::Day7 => Span::new().days(amount),
            Self::Month1 => Span::new().months(amount),
            Self::Year1 => Span::new().years(amount),
        }
    }
}

impl std::fmt::Display for TimeStep {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.value(), self.unit())
    }
}

impl std::str::FromStr for TimeStep {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|step| step.to_string() == s)
            .ok_or_else(|| format!("unknown time step: {s}"))
    }
}

/// Which ends of the time range the stepper controls move.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum StepTarget {
    Start,
    #[default]
    Both,
    End,
}

impl StepTarget {
    pub fn moves_start(&self) -> bool {
        matches!(self, Self::Start | Self::Both)
    }

    pub fn moves_end(&self) -> bool {
        matches!(self, Self::End | Self::Both)
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
            // The snapping to day start/end adds an extra day, so 6d keeps the
            // range at 1 week long.
            start_offset: -Span::new().days(6),
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

    /// Show the day containing the given instant, snapped to the day.
    /// Which day contains the instant depends on the day separation time,
    /// applied by `to_time_range`.
    pub fn day_containing(ts: Timestamp) -> Result<Self, jiff::Error> {
        let delta = Timestamp::now().until(ts)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    const TZ_NAME: &str = "America/New_York";

    fn day_of(dt: jiff::civil::DateTime, sep: Time) -> TimeRange {
        let ts = dt.intz(TZ_NAME).unwrap().timestamp();
        let range = TimeDeltaRange::day_containing(ts)
            .unwrap()
            .to_time_range(TZ_NAME, sep)
            .unwrap();
        assert!(range.contains(&ts));
        range
    }

    #[test]
    fn day_containing_after_separator() {
        let sep = Time::constant(4, 0, 0, 0);
        let range = day_of(date(2023, 6, 15).at(13, 0, 0, 0), sep);
        let start = range.start.intz(TZ_NAME).unwrap();
        let end = range.end.intz(TZ_NAME).unwrap();
        assert_eq!(start.datetime(), date(2023, 6, 15).at(4, 0, 0, 0));
        assert_eq!(end.datetime(), date(2023, 6, 16).at(4, 0, 0, 0));
    }

    #[test]
    fn day_containing_before_separator() {
        let sep = Time::constant(4, 0, 0, 0);
        // 01:30 is before the 04:00 separator, so it belongs to the day
        // starting the previous calendar date
        let range = day_of(date(2023, 6, 15).at(1, 30, 0, 0), sep);
        let start = range.start.intz(TZ_NAME).unwrap();
        let end = range.end.intz(TZ_NAME).unwrap();
        assert_eq!(start.datetime(), date(2023, 6, 14).at(4, 0, 0, 0));
        assert_eq!(end.datetime(), date(2023, 6, 15).at(4, 0, 0, 0));
    }

    fn zdt(dt: jiff::civil::DateTime) -> Timestamp {
        dt.intz(TZ_NAME).unwrap().timestamp()
    }

    #[test]
    fn stepped_day_over_dst() {
        // 2024-03-10 is the US spring-forward date
        let range = TimeRange {
            start: zdt(date(2024, 3, 9).at(12, 0, 0, 0)),
            end: zdt(date(2024, 3, 9).at(18, 0, 0, 0)),
        };
        let stepped = range
            .stepped(TimeStep::Day1.span(1), StepTarget::Both, TZ_NAME)
            .unwrap()
            .unwrap();
        // lands on the same wall-clock times despite the 23-hour day
        assert_eq!(
            stepped.start.intz(TZ_NAME).unwrap().datetime(),
            date(2024, 3, 10).at(12, 0, 0, 0)
        );
        assert_eq!(
            stepped.end.intz(TZ_NAME).unwrap().datetime(),
            date(2024, 3, 10).at(18, 0, 0, 0)
        );
        assert_eq!(
            stepped.start.as_second() - range.start.as_second(),
            23 * 3600
        );
    }

    #[test]
    fn stepped_month_clamps_to_month_end() {
        let range = TimeRange {
            start: zdt(date(2024, 1, 31).at(10, 0, 0, 0)),
            end: zdt(date(2024, 1, 31).at(12, 0, 0, 0)),
        };
        let stepped = range
            .stepped(TimeStep::Month1.span(1), StepTarget::Both, TZ_NAME)
            .unwrap()
            .unwrap();
        assert_eq!(
            stepped.start.intz(TZ_NAME).unwrap().datetime(),
            date(2024, 2, 29).at(10, 0, 0, 0)
        );
    }

    #[test]
    fn stepped_moves_only_the_target() {
        let range = TimeRange {
            start: zdt(date(2024, 6, 1).at(0, 0, 0, 0)),
            end: zdt(date(2024, 6, 2).at(0, 0, 0, 0)),
        };
        let stepped = range
            .stepped(TimeStep::Hour1.span(-2), StepTarget::Start, TZ_NAME)
            .unwrap()
            .unwrap();
        assert_eq!(
            stepped.start.intz(TZ_NAME).unwrap().datetime(),
            date(2024, 5, 31).at(22, 0, 0, 0)
        );
        assert_eq!(stepped.end, range.end);
    }

    #[test]
    fn stepped_start_cannot_cross_end() {
        let range = TimeRange {
            start: zdt(date(2024, 6, 1).at(0, 0, 0, 0)),
            end: zdt(date(2024, 6, 1).at(0, 30, 0, 0)),
        };
        let stepped = range
            .stepped(TimeStep::Hour1.span(1), StepTarget::Start, TZ_NAME)
            .unwrap();
        assert_eq!(stepped, None);
    }

    #[test]
    fn day_containing_midnight_separator() {
        let range = day_of(date(2023, 6, 15).at(13, 0, 0, 0), Time::MIN);
        let start = range.start.intz(TZ_NAME).unwrap();
        let end = range.end.intz(TZ_NAME).unwrap();
        assert_eq!(start.datetime(), date(2023, 6, 15).at(0, 0, 0, 0));
        assert_eq!(end.datetime(), date(2023, 6, 16).at(0, 0, 0, 0));
    }
}
