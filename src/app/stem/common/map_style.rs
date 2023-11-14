//! Types that define the map style, such as marker and basemap appearance.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    cmaps::{Cmap, CmapParams},
    float,
    state::ok_or_default,
    units::UnitPreference,
    Location,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MapStyle {
    pub solid_color: Rgba,
    pub marker_size: usize,
    pub line_size: usize,
    pub basemap_style: BasemapStyle,
    pub colored_datastream: ColoredDataStream,
    pub show_colorbar: bool,
    #[serde(deserialize_with = "ok_or_default")]
    pub automap: bool, // hide unexplored map regions
}

impl MapStyle {
    pub fn should_show_colorbar(&self) -> bool {
        self.show_colorbar
            && self.colored_datastream.is_some()
            && self.colored_datastream != ColoredDataStream::Time
    }
}

impl Default for MapStyle {
    fn default() -> Self {
        Self {
            solid_color: Rgba {
                rgb: String::from("#0a84ff"),
                a: 1.0,
            },
            marker_size: 3,
            line_size: 2,
            basemap_style: Default::default(),
            colored_datastream: Default::default(),
            show_colorbar: true,
            automap: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub rgb: String, // hex code
    pub a: f64,
}

// === BASEMAP STYLES === //

/// Displayable enum for basemap selections
///
/// Consists of public tile server options available in plotly natively
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum BasemapStyle {
    Basic,
    Dataviz,
    Streets,
    Topo,
    Outdoor,
    #[default]
    BasicDark,
    DatavizDark,
    StreetsDark,
    TopoDark,
    OutdoorDark,
    Satellite,
}

impl BasemapStyle {
    /// Returns true if the style is a dark theme
    /// We consider sattelite to be a dark theme since its background is black
    pub fn is_dark(&self) -> bool {
        match self {
            Self::BasicDark
            | Self::DatavizDark
            | Self::StreetsDark
            | Self::TopoDark
            | Self::OutdoorDark
            | Self::Satellite => true,
            Self::Basic
            | Self::Dataviz
            | Self::Streets
            | Self::Topo
            | Self::Outdoor => false,
        }
    }
}

// Displays according to order of this array
pub static BASEMAP_STRINGS: [(BasemapStyle, &str); 11] = [
    (BasemapStyle::Basic, "Basic"),
    (BasemapStyle::Dataviz, "Dataviz"),
    (BasemapStyle::Streets, "Streets"),
    (BasemapStyle::Topo, "Topo"),
    (BasemapStyle::Outdoor, "Outdoor"),
    (BasemapStyle::BasicDark, "Dark Basic"),
    (BasemapStyle::DatavizDark, "Dark Dataviz"),
    (BasemapStyle::StreetsDark, "Dark Streets"),
    (BasemapStyle::TopoDark, "Dark Topo"),
    (BasemapStyle::OutdoorDark, "Dark Outdoor"),
    (BasemapStyle::Satellite, "Satellite"),
];

impl fmt::Display for BasemapStyle {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = BASEMAP_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEnumError;

impl std::str::FromStr for BasemapStyle {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = BASEMAP_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}

// === COLORED DATASTREAM === //

/// Displayable enum for datastream selection for colormapping
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum ColoredDataStream {
    #[default]
    None,
    Lat,
    Lon,
    HorizAccuracy,
    Altitude,
    VertAccuracy,
    Story,
    Speed,
    SpeedAccuracy,
    Course,
    CourseAccuracy,
    Time,
    TimeOfDay,
}

pub static DATASTREAM_STRINGS: [(ColoredDataStream, &str); 11] = [
    (ColoredDataStream::None, "None"),
    // (ColoredDataStream::Lat, "Latitude"),
    // (ColoredDataStream::Lon, "Longitude"),
    (ColoredDataStream::Speed, "Speed"),
    (ColoredDataStream::Course, "Course"),
    (ColoredDataStream::Time, "Time"),
    (ColoredDataStream::TimeOfDay, "Time of Day"),
    (ColoredDataStream::Altitude, "Altitude"),
    (ColoredDataStream::Story, "Building Story"),
    (ColoredDataStream::HorizAccuracy, "Horizontal Error"),
    (ColoredDataStream::VertAccuracy, "Altitude Error"),
    (ColoredDataStream::SpeedAccuracy, "Speed Error"),
    (ColoredDataStream::CourseAccuracy, "Course Error"),
];

impl ColoredDataStream {
    /// Selects the right data stream from a common::Location struct
    pub fn get_stream(
        &self,
        loc: &Location,
        offset: &time::UtcOffset,
    ) -> Option<f64> {
        let time_of_day_to_seconds = |timestamp: &time::OffsetDateTime| {
            let hms = timestamp.to_offset(*offset).time().as_hms();
            (hms.0 as f64) * 60. * 60. + (hms.1 as f64) * 60. + (hms.2 as f64)
        };
        match self {
            ColoredDataStream::None => None,
            ColoredDataStream::Lat => Some(loc.latitude),
            ColoredDataStream::Lon => Some(loc.longitude),
            ColoredDataStream::HorizAccuracy => Some(loc.horizontal_accuracy),
            ColoredDataStream::Altitude => loc.msl_altitude,
            ColoredDataStream::VertAccuracy => loc.vertical_accuracy,
            ColoredDataStream::Story => loc.story.map(|s| s as f64),
            ColoredDataStream::Speed => loc.speed,
            ColoredDataStream::SpeedAccuracy => loc.speed_accuracy,
            ColoredDataStream::Course => loc.course,
            ColoredDataStream::CourseAccuracy => loc.course_accuracy,
            ColoredDataStream::Time => {
                Some(loc.timestamp.unix_timestamp() as f64)
            }
            ColoredDataStream::TimeOfDay => {
                Some(time_of_day_to_seconds(&loc.timestamp))
            }
        }
    }

    /// Returns true if the data is to be colormapped
    pub fn is_some(&self) -> bool {
        *self != Self::None
    }

    pub fn name_with_unit(&self, unit_pref: &UnitPreference) -> String {
        match *self {
            // No units, just display name
            ColoredDataStream::None
            | ColoredDataStream::Time
            | ColoredDataStream::TimeOfDay
            | ColoredDataStream::Story => format!("{}", self),
            // Degree units
            ColoredDataStream::Lat
            | ColoredDataStream::Lon
            | ColoredDataStream::Course
            | ColoredDataStream::CourseAccuracy => {
                format!("{} (º)", self)
            }
            // Small lengths
            ColoredDataStream::HorizAccuracy
            | ColoredDataStream::Altitude
            | ColoredDataStream::VertAccuracy => {
                format!("{} ({})", self, unit_pref.small_length.abbreviation())
            }
            // Speeds
            ColoredDataStream::Speed | ColoredDataStream::SpeedAccuracy => {
                format!("{} ({})", self, unit_pref.velocity.abbreviation())
            }
        }
    }

    /// Calculate the cmin and cmax and return colormap the for a datastream
    /// given a vec of locations
    pub fn get_cmap_params(
        &self,
        records: &[Location],
        offset: &time::UtcOffset,
    ) -> CmapParams {
        let mut params = CmapParams {
            cmin: 0.,
            cmax: 0.,
            cmap: Cmap::Plasma,
        };
        if !self.is_some() {
            // no data-based color mapping, just short circuit
            return params;
        }
        if *self == ColoredDataStream::Course {
            params.cmax = 360.;
            params.cmap = Cmap::Twilight;
        } else if *self == ColoredDataStream::TimeOfDay {
            params.cmax = 24. * 60. * 60.;
            params.cmap = Cmap::TwilightShifted;
        } else {
            let colorvals: Vec<_> = records
                .iter()
                .filter_map(|x| self.get_stream(x, offset))
                .collect();
            params.cmin = float::min(&colorvals);
            params.cmax = float::max(&colorvals);
        }
        params
    }
}

impl fmt::Display for ColoredDataStream {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = DATASTREAM_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

impl std::str::FromStr for ColoredDataStream {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = DATASTREAM_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}
