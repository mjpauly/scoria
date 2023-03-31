//! Picker element for marker color and opacity, basemap style

use std::fmt;
use std::str::FromStr;

use plotly::layout::MapboxStyle;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

/// Our version derives PartialEq so it can be used in yew hooks
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl Rgba {
    pub fn to_plotly(&self) -> plotly::color::Rgba {
        plotly::color::Rgba::new(self.r, self.g, self.b, self.a)
    }
}

/// Displayable enum for basemap selections
///
/// Consists of public tile server options available in plotly natively
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum BasemapStyle {
    CartoDarkMatter,
    CartoPositron,
    OpenStreetMap,
    StamenTerrain,
    StamenToner,
    StamenWatercolor,
    WhiteBg,
}

static BASEMAP_STRINGS: [(BasemapStyle, &'static str); 7] = [
    (BasemapStyle::CartoDarkMatter, "CartoDarkMatter"),
    (BasemapStyle::CartoPositron, "CartoPositron"),
    (BasemapStyle::OpenStreetMap, "OpenStreetMap"),
    (BasemapStyle::StamenTerrain, "StamenTerrain"),
    (BasemapStyle::StamenToner, "StamenToner"),
    (BasemapStyle::StamenWatercolor, "StamenWatercolor"),
    (BasemapStyle::WhiteBg, "WhiteBg"),
];

impl BasemapStyle {
    pub fn to_plotly(&self) -> MapboxStyle {
        match self {
            BasemapStyle::CartoDarkMatter => MapboxStyle::CartoDarkMatter,
            BasemapStyle::CartoPositron => MapboxStyle::CartoPositron,
            BasemapStyle::OpenStreetMap => MapboxStyle::OpenStreetMap,
            BasemapStyle::StamenTerrain => MapboxStyle::StamenTerrain,
            BasemapStyle::StamenToner => MapboxStyle::StamenToner,
            BasemapStyle::StamenWatercolor => MapboxStyle::StamenWatercolor,
            BasemapStyle::WhiteBg => MapboxStyle::WhiteBg,
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

// for colorscales we need to embed an array of values in the marker, so we
// don't construct marker instances here, but bubble up the config values

#[derive(Properties, PartialEq)]
pub struct MapStylerProps {
    pub solid_color: UseStateHandle<Rgba>,
    pub marker_size: UseStateHandle<usize>,
    pub basemap_style: UseStateHandle<BasemapStyle>,
    // pub colorscale: ... // solid, viridis, inferno, etc
    // pub colorscale_value: ... // lat, lon, speed,
}

use std::u8;
#[function_component]
pub fn MapStyler(
    MapStylerProps {
        solid_color,
        marker_size,
        basemap_style,
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

    // html formatting
    let color_string = format!(
        "#{:02x}{:02x}{:02x}",
        solid_color.r, solid_color.g, solid_color.b
    );
    // default option should be put first
    let basemap_options = BASEMAP_STRINGS.iter().map(|x| {
        html! { <option>{x.1.to_string()}</option> }
    });

    html! {
        // <div class="overflow-y-auto my-2 mx-2 grid grid-cols-2 items-center \
                    // justify-items-start auto-cols-auto">
        <div class="overflow-y-auto my-1 flex">
        <div class="max-w-fit mx-auto">
            <div class="flex items-center justify-between">
                <label for="marker_color">{"Marker Color"}</label>
                <input type="color" id="marker_color" value={color_string}
                    class="m-1 ml-6"
                    onchange={color_onchange} />
            </div>
            <div class="flex items-center justify-between">
                <label for="opacity">{"Marker Opacity"}</label>
                <input type="range" id="opacity" value={format!("{}", solid_color.a)}
                    min="0.0" max="1.0" step="0.01" class="m-1 ml-6"
                    onchange={opacity_onchange} />
            </div>
            <div class="flex items-center justify-between">

                <label for="marker_size">{"Marker Size"}</label>
                <input type="range" id="marker_size"
                    value={format!("{}", **marker_size)}
                    min="1" max="20" class="m-1 ml-6"
                    onchange={size_onchange} />
            </div>
            <div class="flex items-center justify-between">
                <label for="basemap">{"Basemap Style"}</label>
                <select onchange={basemap_onchange} id="basemap"
                    class="m-1 ml-6">
                    {for basemap_options}
                </select>
            </div>
        </div>
        </div>
    }
}
