use serde::{Deserialize, Serialize};
use time::{macros::format_description, OffsetDateTime};

// use crate::state::ok_or_default;

/// User time preferences.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimePreference {
    pub twelve_hour_clock: bool, // use 12-hour clock
}
// #[serde(deserialize_with = "ok_or_default")]
// offset: OffsetPreference,

/// User preference on what UTC offsets to use.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum OffsetPreference {
    #[default]
    Localized, // based on timezone of the location
    Current,                // use current UTC offset
    Fixed(time::UtcOffset), // fixed UTC offset
}

impl TimePreference {
    pub fn format_time(
        &self,
        t: OffsetDateTime,
    ) -> Result<String, time::error::Format> {
        format_datetime(t, None, self.twelve_hour_clock)
    }
    pub fn format_time_with_previous(
        &self,
        t: OffsetDateTime,
        prev: Option<OffsetDateTime>,
    ) -> Result<String, time::error::Format> {
        format_datetime(t, prev, self.twelve_hour_clock)
    }
}

/// Format in a similar style to RFC 2822, but omit components that are obvious
/// from context.
///
/// E.g. for these two dates in a timeline:
///
/// Sun, 01 May 2024 17:15:30 -0700
/// Sun, 01 May 2024 18:21:36 -0700
///
/// The dates and offsets are the same, and with this context we abbreviate the
/// second date to:
///
/// 18:21:36
///
/// We can also format with AM/PM like so:
/// Sun, 01 May 2024 05:15:30 PM -0700
pub fn format_datetime(
    t: OffsetDateTime,
    prev: Option<OffsetDateTime>,
    ampm: bool,
) -> Result<String, time::error::Format> {
    let mut same_date = false;
    let mut same_offset = false;
    if let Some(prev) = prev {
        same_date = t.date() == prev.date();
        same_offset = t.offset() == prev.offset();
    }
    let mut out = String::new();
    if !same_date {
        out.push_str(&t.format(&format_description!(
            "[weekday repr:short], [day] [month repr:short] [year] "
        ))?);
    }
    if ampm {
        out.push_str(&t.format(&format_description!(
            "[hour repr:12]:[minute]:[second] [period]"
        ))?);
    } else {
        out.push_str(
            &t.format(&format_description!("[hour]:[minute]:[second]"))?,
        );
    }
    if !same_offset {
        out.push_str(&t.format(&format_description!(
            " [offset_hour sign:mandatory]:[offset_minute]"
        ))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
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
}
