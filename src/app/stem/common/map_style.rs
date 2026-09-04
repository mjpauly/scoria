//! Types that define the map style, such as marker and basemap appearance.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::cmaps::CmapTrim;
use crate::{state::ok_or_default, units::UnitPreference};

pub const MARKER_SIZE_MIN: f64 = 0.0;
pub const MARKER_SIZE_MAX: f64 = 10.0;

/// Max zoom of the map. Shared with the backend tile slicer, whose point
/// buffer depends on how far a tile can be overzoomed.
pub const MAP_MAXZOOM: i32 = 22;
/// Max zoom of the data tile source; tiles at this zoom are overzoomed up
/// to MAP_MAXZOOM. A tile's 4096 extent quantizes positions to 1/4096 of
/// its width: z14 tiles put points on a ~0.5 m grid, visible as aliasing
/// from z17 in (1 px there, 8 px at z20). z20 tiles quantize to ~9 mm, the
/// grid resolution of the data itself. Slicing cost per tile is the same
/// at any zoom.
pub const MVT_SOURCE_MAXZOOM: i32 = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MapStyle {
    pub solid_color: Rgba,
    pub marker_size: f64,
    pub line_size: usize,
    #[serde(deserialize_with = "ok_or_default")]
    pub basemap_style: BasemapStyle,
    #[serde(deserialize_with = "ok_or_default")]
    pub colored_datastream: ColoredDataStream,
    pub show_colorbar: bool,
    pub automap: bool, // hide unexplored map regions
    pub automap_opacity: f64,
    pub show_last_location: bool,
    pub pins_below_data: bool, // display pins below log data
    pub hide_points_outside_viewbounds: bool,
    #[serde(deserialize_with = "ok_or_default")]
    pub decimation_mode: DecimationMode,
    #[serde(deserialize_with = "ok_or_default")]
    pub temporal_max_points: TemporalMaxPoints,
    #[serde(deserialize_with = "ok_or_default")]
    pub spatial_cell_pitch: SpatialCellPitch,
    #[serde(deserialize_with = "ok_or_default")]
    pub contrast_reserve: ContrastReserve,
}

/// How much of the colormap to leave unused so data colors don't blend into
/// the basemap.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum ContrastReserve {
    Off,
    #[default]
    #[strum(serialize = "10%")]
    TenPercent,
    #[strum(serialize = "20%")]
    TwentyPercent,
    #[strum(serialize = "30%")]
    ThirtyPercent,
}

impl ContrastReserve {
    pub fn fraction(&self) -> f64 {
        match self {
            Self::Off => 0.,
            Self::TenPercent => 0.1,
            Self::TwentyPercent => 0.2,
            Self::ThirtyPercent => 0.3,
        }
    }
}

/// Pixel pitch of spatial decimation cells. Independent of marker size: even
/// large markers whose centers are offset by a small fraction of their radius
/// produce visible differences, so tracks look lumpy unless the pitch is near
/// pixel scale. Coarser pitches return fewer points (query time is
/// scan-dominated and barely responds to pitch, ~13% from 0.5 to 4 px).
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum SpatialCellPitch {
    #[default]
    #[strum(serialize = "0.5 px")]
    HalfPx,
    #[strum(serialize = "1 px")]
    OnePx,
    #[strum(serialize = "2 px")]
    TwoPx,
    #[strum(serialize = "4 px")]
    FourPx,
}

impl SpatialCellPitch {
    pub fn pixels(&self) -> f64 {
        match self {
            Self::HalfPx => 0.5,
            Self::OnePx => 1.,
            Self::TwoPx => 2.,
            Self::FourPx => 4.,
        }
    }
}

/// How to thin the map query result when it exceeds the point budget. Both
/// modes are detail-vs-latency settings, not crash prevention
/// (doc/decimation/memory-limits.md). Spatial is the default: measured
/// on-phone it matches temporal query latency at 0.5 px and looks far
/// better at large N; temporal is for faster queries and for lines.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum DecimationMode {
    /// Keep the most recent point per screen-space cell, so busy areas don't
    /// crowd out sparse ones. Lines aren't drawn in this mode: joining
    /// points that may be far apart in time would falsely suggest tracks
    /// connect directly across out-of-bounds travel.
    #[default]
    Spatial,
    /// Keep every nth point, up to a max count. The sample is time-uniform,
    /// so lines can be drawn between consecutive points.
    Temporal,
}

/// Max points kept by temporal decimation, its detail-vs-latency knob.
/// More points show finer time detail; fewer query and draw faster. An
/// unlimited option was tried and dropped: 1.1M points made the phone
/// map barely usable, and spatial decimation is the better way to see
/// everything.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum TemporalMaxPoints {
    #[strum(serialize = "5,000")]
    FiveK,
    #[strum(serialize = "10,000")]
    TenK,
    #[strum(serialize = "20,000")]
    TwentyK,
    #[strum(serialize = "50,000")]
    FiftyK,
    // Default calibrated on an iPhone 16 Pro: ~800 ms per 100k retrieved
    // before the narrow row fetch, ~half that with it.
    #[default]
    #[strum(serialize = "100,000")]
    HundredK,
}

impl TemporalMaxPoints {
    pub fn points(&self) -> u64 {
        match self {
            Self::FiveK => 5_000,
            Self::TenK => 10_000,
            Self::TwentyK => 20_000,
            Self::FiftyK => 50_000,
            Self::HundredK => 100_000,
        }
    }
}

impl MapStyle {
    /// The colormap trim that preserves contrast against the current basemap:
    /// reserve the bright end of the scale on light basemaps and the dark end
    /// on dark ones. Satellite imagery sits mid-brightness, so neither end is
    /// trimmed there.
    pub fn cmap_trim(&self) -> CmapTrim {
        let t = self.contrast_reserve.fraction();
        if t == 0. {
            return CmapTrim::None;
        }
        match self.basemap_style.darkness() {
            BasemapDarkness::Light => CmapTrim::BrightEnd(t),
            BasemapDarkness::Dark => CmapTrim::DarkEnd(t),
            BasemapDarkness::Neither => CmapTrim::None,
        }
    }

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
            marker_size: 3.0,
            line_size: 2,
            basemap_style: Default::default(),
            colored_datastream: Default::default(),
            show_colorbar: true,
            automap: false,
            automap_opacity: 0.5,
            show_last_location: true,
            pins_below_data: false,
            hide_points_outside_viewbounds: false,
            decimation_mode: Default::default(),
            temporal_max_points: Default::default(),
            spatial_cell_pitch: Default::default(),
            contrast_reserve: Default::default(),
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
    None,
    Basic,
    Dataviz,
    Streets,
    Topo,
    Outdoor,
    #[strum(serialize = "Dark None")]
    NoneDark,
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

/// Brightness class of a basemap, deciding which end of a colormap blends
/// into it. Satellite imagery is Neither: its brightness sits mid-range.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BasemapDarkness {
    Light,
    Dark,
    Neither,
}

impl BasemapStyle {
    pub fn darkness(&self) -> BasemapDarkness {
        match self {
            Self::Hybrid | Self::Satellite => BasemapDarkness::Neither,
            Self::NoneDark
            | Self::BasicDark
            | Self::DatavizDark
            | Self::StreetsDark
            | Self::TopoDark
            | Self::OutdoorDark => BasemapDarkness::Dark,
            Self::None
            | Self::Basic
            | Self::Dataviz
            | Self::Streets
            | Self::Topo
            | Self::Outdoor => BasemapDarkness::Light,
        }
    }

    /// Returns true if the style is a dark theme
    /// We consider sattelite to be a dark theme since its background is black
    pub fn is_dark(&self) -> bool {
        matches!(
            self.darkness(),
            BasemapDarkness::Dark | BasemapDarkness::Neither
        )
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None | Self::NoneDark)
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

    // These colors are based o data from multiple points
    #[strum(serialize = "Short Dwell Detection")]
    ShortDwellDetection,
    #[strum(serialize = "Long Dwell Detection")]
    LongDwellDetection,
    #[strum(serialize = "Dwell Score")]
    DwellScore,
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
            Self::ShortDwellDetection
            | Self::LongDwellDetection
            | Self::DwellScore => false,
        }
    }

    /// Return true if the coloring relies on "adjacent" points that are the
    /// next point just outside the view bounds wherever the track goes outside.
    ///
    /// These adjacent points are marked as not being used for the colormapping,
    /// and this marking helps certain colormapping schemes better associate
    /// related points and produce an accurate colormap.
    pub fn should_get_adjacent(&self) -> bool {
        matches!(self, Self::LongDwellDetection | Self::DwellScore)
    }

    /// Return true if the coloring is computed from neighboring points in
    /// time, so it needs a time-contiguous sample rather than a spatial one.
    pub fn uses_adjacent_points(&self) -> bool {
        matches!(
            self,
            Self::ShortDwellDetection
                | Self::LongDwellDetection
                | Self::DwellScore
        )
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
            | Self::LongDwellDetection
            | Self::DwellScore => format!("{}", self),
            // Degree units
            Self::Course | Self::CourseAccuracy => {
                format!("{} (º)", self)
            }
            // Small lengths
            Self::HorizAccuracy | Self::Altitude | Self::VertAccuracy => {
                format!("{} ({})", self, unit_pref.small_length.abbreviation())
            }
            // Speeds
            Self::Speed | Self::SpeedAccuracy => {
                format!("{} ({})", self, unit_pref.velocity.abbreviation())
            }
        }
    }

    pub fn to_preferred_units(
        &self,
        unit_pref: &UnitPreference,
        val: f64,
    ) -> f64 {
        match *self {
            // Small lengths
            Self::HorizAccuracy | Self::Altitude | Self::VertAccuracy => {
                unit_pref.small_length.from_base_unit(val)
            }
            Self::Speed | Self::SpeedAccuracy => {
                unit_pref.velocity.from_base_unit(val)
            }
            _ => val,
        }
    }
}
