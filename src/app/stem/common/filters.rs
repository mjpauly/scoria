//! Types for filtering location data based on thresholding.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::units::{LengthUnits, UnitPreference};
use crate::Location;

/// A filter setting. Determines if data points should be excluded based on
/// whether the values in DataStream when compared with the threshold using the
/// operator returns true.
///
/// E.g. (datastream, op, threshold) of:
/// (DataStream::HorizAccuracy, FilterOp::GreaterThan, 20.0)
/// would exclude data where the horizontal accuracy is worse than 20 meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub id: usize,
    pub enabled: bool, // quick toggle on/off
    pub datastream: DataStream,
    pub op: FilterOp,
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterOp {
    GreaterThan,
    LessThan,
    GreatherThanOrEq,
    LessThanOrEq,
    IsEq,
    IsNotEq,
}

impl Filter {
    /// Returns true if we should remove the location and false if we should
    /// keep it.
    pub fn should_remove(&self, loc: &Location) -> bool {
        if !self.enabled {
            // shouldn't remove the location if the filter isn't enabled
            return false;
        }
        let val = self.datastream.get_stream(loc);
        let threshold = self.threshold;
        match self.op {
            FilterOp::GreaterThan => val > threshold,
            FilterOp::LessThan => val < threshold,
            FilterOp::GreatherThanOrEq => val >= threshold,
            FilterOp::LessThanOrEq => val <= threshold,
            FilterOp::IsEq => val == threshold,
            FilterOp::IsNotEq => val != threshold,
        }
    }
}

/// Apply a set of filters to a slice of locations. Data is filtered out if ANY
/// filter condition's `should_remove` method returns true.
///
/// Returns a vector since we need to collect the filter or the closure lives
/// too long.
pub fn apply_filters<'a>(
    filters: &[Filter],
    records: &'a [Location],
) -> Vec<&'a Location> {
    records
        .iter()
        .filter(|loc| {
            for filt in filters {
                if filt.should_remove(loc) {
                    // invert condition, since filter discards on `false`
                    return false;
                }
            }
            true
        })
        .collect()
}

pub static OP_STRINGS: [(FilterOp, &str); 6] = [
    (FilterOp::GreaterThan, ">"),
    (FilterOp::LessThan, "<"),
    (FilterOp::GreatherThanOrEq, "≥"),
    (FilterOp::LessThanOrEq, "≤"),
    (FilterOp::IsEq, "="),
    (FilterOp::IsNotEq, "≠"),
];

impl fmt::Display for FilterOp {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = OP_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEnumError;

impl std::str::FromStr for FilterOp {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item =
            OP_STRINGS.iter().find(|x| x.1 == s).ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}

// === DATASTREAM === //

// Displayable enum for datastream selection and parsing/formatting of values
// with correct units.

use uom::str::ParseQuantityError;

/// All possible data streams, excluding time which is a fairly special case
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataStream {
    Lat,
    Lon,
    HorizAccuracy,
    Speed,
    Course,
}

pub static DATASTREAM_STRINGS: [(DataStream, &str); 5] = [
    (DataStream::Lat, "Lat"),
    (DataStream::Lon, "Lon"),
    (DataStream::HorizAccuracy, "H Accuracy"),
    (DataStream::Speed, "Speed"),
    (DataStream::Course, "Course"),
];

// Stream-specific conversions

impl DataStream {
    /// Selects the right data stream from a common::Location struct
    pub fn get_stream(&self, loc: &Location) -> f64 {
        match self {
            DataStream::Lat => loc.lat,
            DataStream::Lon => loc.lon,
            DataStream::HorizAccuracy => loc.accuracy,
            DataStream::Speed => loc.speed,
            DataStream::Course => loc.course,
        }
    }

    /// Parse a string such as "1.0 m" or "5 ft" into a quantity, then into f64
    /// as the base unit (meters for length)
    pub fn parse_value(
        &self,
        unit_pref: &UnitPreference,
        val: &str,
    ) -> Result<f64, ParseQuantityError> {
        match self {
            DataStream::Lat => unit_pref.parse_angle(val),
            DataStream::Lon => unit_pref.parse_angle(val),
            DataStream::HorizAccuracy => unit_pref.parse_small_length(val),
            DataStream::Speed => unit_pref.parse_velocity(val),
            DataStream::Course => unit_pref.parse_angle(val),
        }
    }

    /// Format a value to a string with units corresponding to the stream type.
    pub fn format_value(&self, unit_pref: &UnitPreference, val: f64) -> String {
        match self {
            DataStream::Lat => unit_pref.format_angle(val, None),
            DataStream::Lon => unit_pref.format_angle(val, None),
            DataStream::HorizAccuracy => {
                // Troubles with imprecise displaying on feet, e.g. 29.9999999..
                // so we cap the precision
                match unit_pref.small_length {
                    LengthUnits::Foot => {
                        unit_pref.format_small_length(val, Some(1))
                    }
                    _ => unit_pref.format_small_length(val, None),
                }
            }
            DataStream::Speed => unit_pref.format_velocity(val, None),
            DataStream::Course => unit_pref.format_angle(val, None),
        }
    }
}

// Convert the DataStream enum itself to/from String representation

impl fmt::Display for DataStream {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = DATASTREAM_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

impl std::str::FromStr for DataStream {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = DATASTREAM_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}
