//! Backend time zone lookup routines.
//!
//! On Android the system time zone and using the Time Zone Database require a
//! different API.

use crate::app_state::{get_front_state, set_back_state};
use anyhow::Result;
use common::{units::time::TimeZonePreference, FrontState, LngLat, Location};
use jiff::{tz::TimeZone, Timestamp, Zoned};
use once_cell::sync::Lazy;
use tzf_rs::DefaultFinder;

pub fn update_map_tz(old_front: &Option<FrontState>) {
    let map_center = get_front_state(|s| s.map.view_pos.center);
    let time_pref =
        get_front_state(|s| s.time_pref.clone()).unwrap_or_default();
    let tz_pref_same = Some(time_pref.tz_pref)
        == old_front.as_ref().map(|s| s.time_pref.tz_pref);
    if tz_pref_same
        && time_pref.tz_pref == TimeZonePreference::Localized
        && map_center == old_front.as_ref().map(|s| s.map.view_pos.center)
    {
        // Same localized map center -> don't fetch tz name
        return;
    }
    let tz = match time_pref.tz_pref {
        TimeZonePreference::Localized => {
            map_center.map(|c| get_location_tz_name(c).to_string())
        }
        TimeZonePreference::Current => get_system_tz_name().ok(),
        TimeZonePreference::Fixed => Some(time_pref.fixed_tz.clone()),
    };

    if let Some(new_tz) = tz {
        set_back_state(|s| s.map_tz = common::time_range::TZ(new_tz));
    }
}

pub fn datetime_fn_infallible() -> impl Fn(&Location) -> Zoned {
    let f = location_datetime_fn();
    move |l| {
        f(l).unwrap_or_else(|_| {
            jiff::Timestamp::from_nanosecond(l.timestamp.unix_timestamp_nanos())
                .unwrap()
                .to_zoned(jiff::tz::TimeZone::UTC)
        })
    }
}

/// Makes a function that takes a Location and returns the Zoned datetime
/// according to the user's time zone preference.
///
/// Efforts are made to minimize the amount of work done in the closure, which
/// runs much more often.
pub fn location_datetime_fn() -> impl Fn(&Location) -> Result<Zoned> {
    let pref = get_front_state(|s| s.time_pref.clone()).unwrap_or_default();
    // Some(tz) if a single time zone, and None if localized.
    // Failure fallbacks:
    // - Localized -> Error
    // - System -> Localized
    // - Fixed -> UTC
    let single_tz = match pref.tz_pref {
        TimeZonePreference::Localized => None,
        TimeZonePreference::Current => get_system_tz()
            .map_err(|e| tracing::error!("Failed to get system tz: {e}"))
            .ok(),
        TimeZonePreference::Fixed => Some(
            get_tz(&pref.fixed_tz)
                .map_err(|e| {
                    tracing::error!("Failed to get tz for name, using UTC: {e}")
                })
                .unwrap_or(TimeZone::UTC),
        ),
    };
    // no error logging inside closure, since it runs many times.
    move |l: &Location| {
        let ts = Timestamp::from_nanosecond(l.timestamp.unix_timestamp_nanos())
            .unwrap();
        let tz = match &single_tz {
            Some(tz) => tz.clone(),
            None => get_tz(get_location_tz_name(l.lnglat()))?,
        };
        Ok(ts.to_zoned(tz))
    }
}

pub fn get_location_tz_name(x: LngLat) -> &'static str {
    static FINDER: Lazy<DefaultFinder> = Lazy::new(DefaultFinder::new);
    FINDER.get_tz_name(x.lng, x.lat)
}

#[cfg(target_os = "android")]
pub use android::*;
#[cfg(not(target_os = "android"))]
pub use ios::*;

#[cfg(target_os = "android")]
mod android {
    use anyhow::Result;
    use jiff::tz::TimeZone;

    pub fn get_system_tz_name() -> Result<String> {
        Ok(iana_time_zone::get_timezone()?)
    }

    pub fn get_system_tz() -> Result<TimeZone> {
        let tz_name = iana_time_zone::get_timezone()?;
        get_tz(&tz_name)
    }

    /// TODO: consider caching with `cached` crate
    pub fn get_tz(tz_name: &str) -> Result<TimeZone> {
        let tz_data = android_tzdata::find_tz_data(&tz_name)?;
        let tz = TimeZone::tzif(&tz_name, &tz_data)?;
        Ok(tz)
    }
}

#[cfg(not(target_os = "android"))]
mod ios {
    use anyhow::Result;
    use jiff::tz::TimeZone;

    pub fn get_system_tz_name() -> Result<String> {
        TimeZone::system()
            .iana_name()
            .map(|s| s.to_string())
            .ok_or(anyhow::anyhow!("No IANA name for system Time Zone"))
    }

    pub fn get_system_tz() -> Result<TimeZone> {
        Ok(TimeZone::system())
    }

    pub fn get_tz(tz_name: &str) -> Result<TimeZone> {
        Ok(TimeZone::get(tz_name)?)
    }
}
