//! Types that define the map style, such as marker and basemap appearance.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::{state::ok_or_default, units::UnitPreference, Location};

pub const MARKER_SIZE_MIN: usize = 0;
pub const MARKER_SIZE_MAX: usize = 10;

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
    #[serde(deserialize_with = "ok_or_default")]
    pub show_last_location: bool,
    #[serde(deserialize_with = "ok_or_default")]
    pub pins_below_data: bool, // display pins below log data
}

impl MapStyle {
    pub fn should_show_colorbar(&self) -> bool {
        self.show_colorbar
            && self.colored_datastream.is_some()
            && self.colored_datastream != ColoredDataStream::Time
            && self.colored_datastream != ColoredDataStream::ShortDwellDetection
            && self.colored_datastream != ColoredDataStream::LongDwellDetection
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
            show_last_location: true,
            pins_below_data: false,
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
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "title_case")]
pub enum BasemapStyle {
    Basic,
    Dataviz,
    Streets,
    Topo,
    Outdoor,
    #[strum(serialize = "Dark Basic")]
    #[default]
    BasicDark,
    #[strum(serialize = "Dark Dataviz")]
    DatavizDark,
    #[strum(serialize = "Dark Streets")]
    StreetsDark,
    #[strum(serialize = "Dark Topo")]
    TopoDark,
    #[strum(serialize = "Dark Outdoor")]
    OutdoorDark,
    Hybrid,
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
            | Self::Hybrid
            | Self::Satellite => true,
            Self::Basic
            | Self::Dataviz
            | Self::Streets
            | Self::Topo
            | Self::Outdoor => false,
        }
    }
}

// === COLORED DATASTREAM === //

/// Displayable enum for datastream selection for colormapping
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "title_case")]
pub enum ColoredDataStream {
    #[default]
    None,
    #[strum(serialize = "Horizontal Error")]
    HorizAccuracy,
    Altitude,
    #[strum(serialize = "Altitude Error")]
    VertAccuracy,
    Speed,
    #[strum(serialize = "Speed Error")]
    SpeedAccuracy,
    Course,
    #[strum(serialize = "Course Error")]
    CourseAccuracy,
    Time,
    #[strum(serialize = "Time of Day")]
    TimeOfDay,

    // These colors are based on adjacent pairs of points
    #[strum(serialize = "Distance Delta")]
    DistanceDelta,
    #[strum(serialize = "Time Delta")]
    TimeDelta,
    #[strum(serialize = "Average Speed")]
    AvgSpeed,
    #[strum(serialize = "Short Dwell Detection")]
    ShortDwellDetection,
    #[strum(serialize = "Long Dwell Detection")]
    LongDwellDetection,
}

impl ColoredDataStream {
    /// Returns whether the coloring is based on the value of a single location
    /// data point.
    pub fn is_point_coloring(&self) -> bool {
        match self {
            Self::None => false,
            Self::HorizAccuracy
            | Self::Altitude
            | Self::VertAccuracy
            | Self::Speed
            | Self::SpeedAccuracy
            | Self::Course
            | Self::CourseAccuracy
            | Self::Time
            | Self::TimeOfDay => true,
            Self::DistanceDelta
            | Self::TimeDelta
            | Self::AvgSpeed
            | Self::ShortDwellDetection
            | Self::LongDwellDetection => false,
        }
    }
    /// Selects the right data stream from a common::Location struct
    pub fn get_stream(
        &self,
        loc: &Location,
        offset: &time::UtcOffset,
    ) -> Option<f64> {
        // TODO: convert to utc offset at the data point's location
        let time_of_day_to_seconds = |timestamp: &time::OffsetDateTime| {
            let hms = timestamp.to_offset(*offset).time().as_hms();
            (hms.0 as f64) * 60. * 60. + (hms.1 as f64) * 60. + (hms.2 as f64)
        };
        match self {
            Self::None => None,
            Self::HorizAccuracy => Some(loc.horizontal_accuracy),
            Self::Altitude => loc.msl_altitude,
            Self::VertAccuracy => loc.vertical_accuracy,
            Self::Speed => loc.speed,
            Self::SpeedAccuracy => loc.speed_accuracy,
            Self::Course => loc.course,
            Self::CourseAccuracy => loc.course_accuracy,
            Self::Time => Some(loc.timestamp.unix_timestamp() as f64),
            Self::TimeOfDay => Some(time_of_day_to_seconds(&loc.timestamp)),
            // computed on pairs of locations, not on a single point
            Self::DistanceDelta
            | Self::TimeDelta
            | Self::AvgSpeed
            | Self::ShortDwellDetection
            | Self::LongDwellDetection => None,
        }
    }

    /// Returns true if the data is to be colormapped
    pub fn is_some(&self) -> bool {
        *self != Self::None
    }

    pub fn name_with_unit(&self, unit_pref: &UnitPreference) -> String {
        match *self {
            // No units, just display name
            Self::None
            | Self::Time
            | Self::TimeOfDay
            | Self::ShortDwellDetection
            | Self::LongDwellDetection => format!("{}", self),
            // Degree units
            Self::Course | Self::CourseAccuracy => {
                format!("{} (º)", self)
            }
            // Small lengths
            Self::HorizAccuracy
            | Self::Altitude
            | Self::VertAccuracy
            | Self::DistanceDelta => {
                format!("{} ({})", self, unit_pref.small_length.abbreviation())
            }
            // Speeds
            Self::Speed | Self::SpeedAccuracy | Self::AvgSpeed => {
                format!("{} ({})", self, unit_pref.velocity.abbreviation())
            }
            Self::TimeDelta => format!("{self} (s)"),
        }
    }

    pub fn to_preferred_units(
        &self,
        unit_pref: &UnitPreference,
        val: f64,
    ) -> f64 {
        match *self {
            // Small lengths
            Self::HorizAccuracy
            | Self::Altitude
            | Self::VertAccuracy
            | Self::DistanceDelta => unit_pref.small_length.from_base_unit(val),
            Self::Speed | Self::SpeedAccuracy | Self::AvgSpeed => {
                unit_pref.velocity.from_base_unit(val)
            }
            _ => val,
        }
    }
}
