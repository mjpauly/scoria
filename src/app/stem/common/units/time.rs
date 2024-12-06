use jiff::{civil::Date, tz::TimeZone, Zoned};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use strum::{Display, EnumIter, EnumString};
use time::{
    format_description::BorrowedFormatItem, macros::format_description,
    OffsetDateTime,
};

use crate::state::ok_or_default;

/// User time preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimePreference {
    pub twelve_hour_clock: bool, // use 12-hour clock
    #[serde(deserialize_with = "ok_or_default")]
    pub tz_pref: TimeZonePreference,
    pub fixed_tz: String, // when using fixed offset, use this tz name
}

impl Default for TimePreference {
    fn default() -> Self {
        Self {
            twelve_hour_clock: true,
            tz_pref: Default::default(),
            fixed_tz: "UTC".to_string(),
        }
    }
}

/// User preference on what UTC offsets to use.
#[derive(
    Debug,
    Copy,
    Default,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum TimeZonePreference {
    #[default]
    Localized, // use time zone of the location where time is formatted
    Current, // use current system time zone, regardless of location
    Fixed,   // fixed IANA time zone name, determined by companion string
}

impl TimePreference {
    /// Format a datetime, omitting the date and time zone if it's the same as
    /// the previously formatted datetime.
    /// e.g. "Thu, Jan 2, 2020 at 3:30:00pm PDT"
    pub fn format_jiff_datetime(
        &self,
        zdt: &Zoned,
        prev: &Option<Zoned>,
    ) -> Result<String, jiff::Error> {
        let mut same_date = false;
        let mut same_tz = false;
        if let Some(prev) = prev {
            same_date = zdt.date() == prev.date();
            same_tz = zdt.time_zone() == prev.time_zone();
        }
        let mut out = String::new();
        if !same_date {
            // "Thu, Jan 2, 2020 at "
            // write! is infallible on String, so we drop the result
            let _ = write!(out, "{}", zdt.strftime("%a, %b %-d, %Y at "));
        }
        write_time(&mut out, zdt, self.twelve_hour_clock);
        if !same_tz {
            // " PDT"
            let _ = write!(out, "{}", zdt.strftime(" %Z"));
        }
        Ok(out)
    }

    /// Format only the time, but show the time zone if it's different than
    /// 'tz_ref', and a day offset number if the date is different than
    /// 'date_ref'
    pub fn format_jiff_time_with_previous(
        &self,
        zdt: &Zoned,
        tz_ref: &Option<&TimeZone>, // if prev formatted tz different, show tz
        date_ref: &Option<Date>,    // if dates different, show "+1"/"-1"
    ) -> Result<String, jiff::Error> {
        let day_difference = date_ref.map(|d| (zdt.date() - d).get_days());
        let mut same_tz = false;
        if let Some(prev_tz) = tz_ref {
            same_tz = &zdt.time_zone() == prev_tz
        }
        let mut s = String::new();
        write_time(&mut s, zdt, self.twelve_hour_clock);
        if let Some(days) = day_difference {
            if days != 0 {
                s.push_str(&to_superscript(format!("{days:+}")));
            }
        }
        if !same_tz {
            // " PDT"
            let _ = write!(s, "{}", zdt.strftime(" %Z"));
        }
        Ok(s)
    }
}

fn write_time(s: &mut String, zdt: &Zoned, twelve_hour_clock: bool) {
    if twelve_hour_clock {
        // "3:30:00pm"
        let _ = write!(s, "{}", zdt.strftime("%-I:%M:%S%P"));
    } else {
        // "15:30:00"
        let _ = write!(s, "{}", zdt.strftime("%H:%M:%S"));
    }
}

pub fn format_date(date: &Date) -> Result<String, jiff::Error> {
    Ok(date.strftime("%A, %B %-d, %Y").to_string())
}

/// Convert a string-formatted integer to superscript using unicode.
fn to_superscript(mut s: String) -> String {
    let superscripts = [
        ('0', "\u{2070}"),
        ('1', "\u{00B9}"),
        ('2', "\u{00B2}"),
        ('3', "\u{00B3}"),
        ('4', "\u{2074}"),
        ('5', "\u{2075}"),
        ('6', "\u{2076}"),
        ('7', "\u{2077}"),
        ('8', "\u{2078}"),
        ('9', "\u{2079}"),
        ('+', "\u{207A}"),
        ('-', "\u{207B}"),
    ];
    for &(norm, sup) in &superscripts {
        s = s.replace(norm, sup);
    }
    s
}

pub fn format_day_of_week(
    t: OffsetDateTime,
) -> Result<String, time::error::Format> {
    t.format(&format_description!("[weekday repr:long]"))
}

pub const LONG_DAY_OF_WEEK_AND_DATE: &[BorrowedFormatItem<'_>] =
    format_description!("[weekday repr:long], [day] [month repr:long] [year]");

#[cfg(test)]
mod tests {
    /* TODO: update for new jiff routines
    use pretty_assertions::assert_eq;
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_format_datetime() {
        let t = datetime!(2020-01-02 03:04:05 +06:07:08);
        assert_eq!(
            format_datetime(t, None, false).unwrap(),
            "Thu, 02 Jan 2020 03:04:05 +06:07"
        );
        // same date and offset
        let tprev = datetime!(2020-01-02 01:15:00 +06:07:08);
        assert_eq!(format_datetime(t, Some(tprev), false).unwrap(), "03:04:05");
        // different date same offset
        let tprev = datetime!(2020-01-01 01:15:00 +06:07:08);
        assert_eq!(
            format_datetime(t, Some(tprev), false).unwrap(),
            "Thu, 02 Jan 2020 03:04:05"
        );
        // same date different offset
        let tprev = datetime!(2020-01-02 01:15:00 +11:00:00);
        assert_eq!(
            format_datetime(t, Some(tprev), false).unwrap(),
            "03:04:05 +06:07"
        );
        // different date different offset
        let tprev = datetime!(2020-01-01 01:15:00 +11:00:00);
        assert_eq!(
            format_datetime(t, Some(tprev), false).unwrap(),
            "Thu, 02 Jan 2020 03:04:05 +06:07"
        );
    }

    #[test]
    fn test_format_ampm_datetime() {
        let t = datetime!(2020-01-02 13:04:05 +06:07:08);
        assert_eq!(
            format_datetime(t, None, true).unwrap(),
            "Thu, 02 Jan 2020 01:04:05 PM +06:07"
        );
    }
    */
}
