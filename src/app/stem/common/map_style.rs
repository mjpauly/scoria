//! Types that define the map style, such as marker and basemap appearance.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

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
    #[serde(deserialize_with = "ok_or_default")]
    pub show_last_location: bool,
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
            show_last_location: true,
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
    #[strum(serialize = "Building Story")]
    Story,
    Speed,
    #[strum(serialize = "Speed Error")]
    SpeedAccuracy,
    Course,
    #[strum(serialize = "Course Error")]
    CourseAccuracy,
    Time,
    #[strum(serialize = "Time of Day")]
    TimeOfDay,
}

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
            ColoredDataStream::Course | ColoredDataStream::CourseAccuracy => {
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
        records: &[&Location],
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
