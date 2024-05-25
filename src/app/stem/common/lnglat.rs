//! Simple Longitude, Latitude struct with named members to prevent accidental
//! ambiguity.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Longitude and latitude, encoded in degrees. Use .to_radians() for radians.
#[derive(PartialEq, Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}

impl LngLat {
    // Checks if a LngLat has values that are on the globe.
    //
    // A LngLat is valid if the longitude is in [-180, 180] and the latitude is
    // in [-90, 90]. Not all LngLats must adhere to this rule, since longitude
    // values outside this range are usedful for LngLatBounds checking over the
    // antimeridian.
    pub fn is_valid(&self) -> bool {
        self.lng >= -180.0
            && self.lng <= 180.0
            && self.lat >= -90.0
            && self.lat <= 90.0
    }
}

impl From<&LngLat> for Vec<f64> {
    fn from(lnglat: &LngLat) -> Self {
        vec![lnglat.lng, lnglat.lat]
    }
}

impl fmt::Display for LngLat {
    /// Format into a "lat, lng" coordinate string.
    ///
    /// Six decimals gives ~10cm accuracy, which is sufficient for Scoria.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:.*}, {:.*}", 6, self.lat, 6, self.lng)
    }
}

impl std::str::FromStr for LngLat {
    type Err = &'static str;

    /// Parse a lat,lng coordinate string.
    ///
    /// Supports common tuple formats, e.g.:
    /// - 35.0, -94.0
    /// - (35.0, -94.0)
    /// - 35.0° N, -94.0° W
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let has_south = s.contains('S');
        let has_west = s.contains('W');
        let coords = s.replace(|c| !is_coord_component(c), "");
        let mut split = coords.split_terminator(',');
        let lat_str = split.next().ok_or("No lat")?;
        let lng_str = split.next().ok_or("No lng")?;
        let mut lat = lat_str.parse::<f64>().map_err(|_| "Bad lat")?;
        let mut lng = lng_str.parse::<f64>().map_err(|_| "Bad lng")?;
        if has_south {
            lat = -lat
        }
        if has_west {
            lng = -lng
        }
        Ok(LngLat { lng, lat })
    }
}

/// Whethether a character is a coordinate component.
///
/// Coordinate components include the decimal digits 0-9, the decimal '.',
/// the minus sign '-', and the comma separator ','.
fn is_coord_component(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-' || c == ','
}
