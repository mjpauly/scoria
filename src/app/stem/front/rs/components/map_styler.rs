//! Picker element for marker color and opacity, basemap style

use std::fmt;
use std::str::FromStr;
use std::u8;

use plotly::layout::MapboxStyle;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::common::Location;
use crate::components::{RANGE_INPUT_STYLE, SELECT_STYLE};

/// Our version derives PartialEq so it can be used in yew hooks
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl Rgba {
    pub fn as_plotly(&self) -> plotly::color::Rgba {
        plotly::color::Rgba::new(self.r, self.g, self.b, self.a)
    }
}

// BASEMAP STYLES

/// Displayable enum for basemap selections
///
/// Consists of public tile server options available in plotly natively
#[derive(Clone, Debug, PartialEq)]
pub enum BasemapStyle {
    BasicDark,
    DatavizDark,
    StreetsDark,
    TopoDark,
    OutdoorDark,
    BasicLight,
    DatavizLight,
    StreetsLight,
    TopoLight,
    OutdoorLight,
    Satellite,
}

// Displays according to order of this array
static BASEMAP_STRINGS: [(BasemapStyle, &str); 11] = [
    (BasemapStyle::BasicLight, "Basic"),
    (BasemapStyle::DatavizLight, "Dataviz"),
    (BasemapStyle::StreetsLight, "Streets"),
    (BasemapStyle::TopoLight, "Topo"),
    (BasemapStyle::OutdoorLight, "Outdoor"),
    (BasemapStyle::BasicDark, "Dark Basic"),
    (BasemapStyle::DatavizDark, "Dark Dataviz"),
    (BasemapStyle::StreetsDark, "Dark Streets"),
    (BasemapStyle::TopoDark, "Dark Topo"),
    (BasemapStyle::OutdoorDark, "Dark Outdoor"),
    (BasemapStyle::Satellite, "Satellite"),
];

impl BasemapStyle {
    fn format_maptiler_url(style: &str) -> String {
        // get the secrets in the .env file at compile time
        let key = dotenvy_macro::dotenv!(
            "MAPTILER_API_KEY",
            "Maptiler API key must be placed in top-level .env file as \
            MAPTILER_API_KEY={key}"
        );
        format!(
            "https://api.maptiler.com/maps/{}/style.json?key={}",
            style, key
        )
    }

    pub fn to_plotly(&self) -> MapboxStyle {
        match self {
            BasemapStyle::BasicDark => {
                MapboxStyle::Custom(Self::format_maptiler_url("basic-v2-dark"))
            }
            BasemapStyle::DatavizDark => {
                MapboxStyle::Custom(Self::format_maptiler_url("dataviz-dark"))
            }
            BasemapStyle::StreetsDark => MapboxStyle::Custom(
                Self::format_maptiler_url("streets-v2-dark"),
            ),
            BasemapStyle::TopoDark => {
                MapboxStyle::Custom(Self::format_maptiler_url("topo-v2-dark"))
            }
            BasemapStyle::OutdoorDark => MapboxStyle::Custom(
                Self::format_maptiler_url("outdoor-v2-dark"),
            ),
            BasemapStyle::BasicLight => {
                MapboxStyle::Custom(Self::format_maptiler_url("basic-v2"))
            }
            BasemapStyle::DatavizLight => {
                MapboxStyle::Custom(Self::format_maptiler_url("dataviz"))
            }
            BasemapStyle::StreetsLight => {
                MapboxStyle::Custom(Self::format_maptiler_url("streets-v2"))
            }
            BasemapStyle::TopoLight => {
                MapboxStyle::Custom(Self::format_maptiler_url("topo-v2"))
            }
            BasemapStyle::OutdoorLight => {
                MapboxStyle::Custom(Self::format_maptiler_url("outdoor-v2"))
            }
            BasemapStyle::Satellite => {
                MapboxStyle::Custom(Self::format_maptiler_url("hybrid"))
            }
        }
    }
}

impl fmt::Display for BasemapStyle {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = BASEMAP_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseBasemapStyleError;

impl std::str::FromStr for BasemapStyle {
    type Err = ParseBasemapStyleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = BASEMAP_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseBasemapStyleError)?;
        Ok(item.0.clone())
    }
}

// COLORED DATA STREAM

/// Displayable enum for datastream selection for colormapping
#[derive(Clone, Debug, PartialEq)]
pub enum ColoredDataStream {
    None,
    Lat,
    Lon,
    HorizAccuracy,
    Speed,
    Course,
    Time,
}

static DATASTREAM_STRINGS: [(ColoredDataStream, &str); 7] = [
    (ColoredDataStream::None, "None"),
    (ColoredDataStream::Lat, "Lat"),
    (ColoredDataStream::Lon, "Lon"),
    (ColoredDataStream::HorizAccuracy, "HorizAccuracy"),
    (ColoredDataStream::Speed, "Speed"),
    (ColoredDataStream::Course, "Course"),
    (ColoredDataStream::Time, "Time"),
];

impl ColoredDataStream {
    /// Selects the right data stream from a common::Location struct
    pub fn get_stream(&self, loc: &Location) -> f64 {
        match self {
            ColoredDataStream::None => 0.,
            ColoredDataStream::Lat => loc.lat,
            ColoredDataStream::Lon => loc.lon,
            ColoredDataStream::HorizAccuracy => loc.accuracy,
            ColoredDataStream::Speed => loc.speed,
            ColoredDataStream::Course => loc.course,
            ColoredDataStream::Time => loc.datetime.unix_timestamp() as f64,
        }
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

#[derive(Debug, PartialEq, Eq)]
pub struct ParseColoredDataStreamError;

impl std::str::FromStr for ColoredDataStream {
    type Err = ParseColoredDataStreamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = DATASTREAM_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseColoredDataStreamError)?;
        Ok(item.0.clone())
    }
}

// YEW COMPONENT

// for colorscales we need to embed an array of values in the marker, so we
// don't construct marker instances here, but bubble up the config values

#[derive(PartialEq, Clone)]
pub struct MapStyle {
    pub solid_color: UseStateHandle<Rgba>,
    pub marker_size: UseStateHandle<usize>,
    pub basemap_style: UseStateHandle<BasemapStyle>,
    pub colored_datastream: UseStateHandle<ColoredDataStream>,
    // pub colorscale: ... // solid, viridis, inferno, etc
}

#[derive(Properties, PartialEq)]
pub struct MapStylerProps {
    pub map_style: MapStyle,
}

#[function_component]
pub fn MapStyler(
    MapStylerProps {
        map_style:
            MapStyle {
                solid_color,
                marker_size,
                basemap_style,
                colored_datastream,
            },
    }: &MapStylerProps,
) -> Html {
    let color_onchange = {
        let solid_color = solid_color.clone();
        Callback::from(move |e: Event| {
            log::debug!("color onchange");
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_color = Rgba {
                r: u8::from_str_radix(&val[1..3], 16).unwrap(),
                g: u8::from_str_radix(&val[3..5], 16).unwrap(),
                b: u8::from_str_radix(&val[5..7], 16).unwrap(),
                ..*solid_color // preserve alpha
            };
            solid_color.set(new_color);
        })
    };
    let opacity_onchange = {
        let solid_color = solid_color.clone();
        Callback::from(move |e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_opacity = val.to_string().parse::<f64>().unwrap();
            let new_color = Rgba {
                a: new_opacity,
                ..*solid_color // other color values
            };
            solid_color.set(new_color);
        })
    };
    let size_onchange = {
        let marker_size = marker_size.clone();
        Callback::from(move |e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            marker_size.set(val.to_string().parse().unwrap());
        })
    };
    let basemap_onchange = {
        let basemap_style = basemap_style.clone();
        Callback::from(move |e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            basemap_style.set(BasemapStyle::from_str(val).unwrap());
        })
    };
    let datastream_onchange = {
        let colored_datastream = colored_datastream.clone();
        Callback::from(move |e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            colored_datastream.set(ColoredDataStream::from_str(val).unwrap());
        })
    };

    // html formatting
    let color_string = format!(
        "#{:02x}{:02x}{:02x}",
        solid_color.r, solid_color.g, solid_color.b
    );
    let basemap_options = BASEMAP_STRINGS.iter().map(|x| {
        html! {
            <option selected={x.0 == **basemap_style}>
                {x.1.to_string()}
            </option>
        }
    });
    let datastream_options = DATASTREAM_STRINGS.iter().map(|x| {
        html! {
            <option selected={x.0 == **colored_datastream}>
                {x.1.to_string()}
            </option>
        }
    });

    html! {
        <div class="mt-2 mb-1 flex">
        <div class="max-w-fit mx-auto">
            <div class="flex items-center justify-between h-8">
                <label for="opacity">{"Marker Opacity"}</label>
                <input type="range" id="opacity"
                    value={format!("{}", solid_color.a)}
                    min="0.0" max="1.0" step="0.01"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    onchange={opacity_onchange} />
            </div>
            <div class="flex items-center justify-between h-8">
                <label for="marker_size">{"Marker Size"}</label>
                <input type="range" id="marker_size"
                    value={format!("{}", **marker_size)}
                    min="1" max="20"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    onchange={size_onchange} />
            </div>
            <div class="flex items-center justify-between">
                <label for="basemap">{"Basemap Style"}</label>
                <select onchange={basemap_onchange} id="basemap"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for basemap_options}
                </select>
            </div>
            <div class="flex items-center justify-between">
                <label for="datastream">{"Data Coloring"}</label>
                <select onchange={datastream_onchange} id="datastream"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for datastream_options}
                </select>
            </div>
            if **colored_datastream == ColoredDataStream::None {
                <div class="flex items-center justify-between">
                    <label for="marker_color">{"Marker Color"}</label>
                    <input type="color" id="marker_color" value={color_string}
                        class="m-1 ml-6 bg-neutral-800"
                        onchange={color_onchange} />
                </div>
            }
        </div>
        </div>
    }
}
