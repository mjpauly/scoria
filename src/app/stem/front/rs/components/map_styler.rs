//! Picker element for marker color and opacity, basemap style

use std::str::FromStr;
use std::u8;

use gloo_net::http::Request;
use obfstr::obfstr;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{RANGE_INPUT_STYLE, SELECT_STYLE, TOGGLE_SWITCH_STYLE};
use crate::ui_state::FrontState;
use common::map_style::{
    BasemapStyle, ColoredDataStream, BASEMAP_STRINGS, DATASTREAM_STRINGS,
};

// BASEMAP STYLES

/// Hook for checking if the scoria tile server is up. If it is, we update the
/// frontend state to get tiles from it.
#[hook]
pub fn use_check_scoria_tile_server() {
    let dispatch = Dispatch::<FrontState>::new();
    yew::platform::spawn_local(async move {
        let result = Request::get(obfstr!("https://api.epsln.com/maps_ok"))
            .send()
            .await;
        if let Ok(resp) = result {
            dispatch.reduce_mut(|s: &mut FrontState| {
                s.use_scoria_tile_server = resp.status() == 200
            });
        } else {
            dispatch.reduce_mut(|s: &mut FrontState| {
                s.use_scoria_tile_server = false
            });
        }
    });
}

/// Get the maptiler key. In this case it's the distribution key, which is more
/// protected than the development key.
#[cfg(feature = "distribution_key")]
fn maptiler_key() -> String {
    obfstr! {
        let maptiler_key = dotenvy_macro::dotenv!(
            "IOS_MAPTILER_API_KEY",
            "Maptiler API key must be placed in top-level .env file as \
            MAPTILER_API_KEY={key}"
        );
    }
    maptiler_key.to_string()
}

#[cfg(not(feature = "distribution_key"))]
fn maptiler_key() -> String {
    obfstr! {
        let maptiler_key = dotenvy_macro::dotenv!(
            "DEV_MAPTILER_API_KEY",
            "Maptiler API key must be placed in top-level .env file as \
            MAPTILER_API_KEY={key}"
        );
    }
    maptiler_key.to_string()
}

fn format_tile_url(style: &str, use_scoria: bool) -> String {
    // get the secrets in the .env file at compile time and obfuscate them
    obfstr! {
        let scoria_key = dotenvy_macro::dotenv!(
            "SCORIA_TILE_API_KEY",
            "Scoria API key must be placed in top-level .env file as \
            SCORIA_TILE_API_KEY={key}"
        );
        let maptiler_base_url = "https://api.maptiler.com/maps";
        let scoria_base_url = "https://api.epsln.com/maps";
        let style_json = "style.json?key=";
    }
    let maptiler_key = maptiler_key();
    let is_satellite = style.contains("hybrid");
    // still use maptiler for satellite images, even if use_scoria is true
    let (base_url, key) = if use_scoria && !is_satellite {
        (scoria_base_url, scoria_key)
    } else {
        (maptiler_base_url, maptiler_key.as_str())
    };
    // should be https://api.url.com/maps/basic-v2/style.json?key=deadbeef
    format!("{base_url}/{style}/{style_json}{key}")
}

pub fn get_basemap_url(style: &BasemapStyle, use_scoria: bool) -> String {
    match style {
        BasemapStyle::Basic => format_tile_url("basic-v2", use_scoria),
        BasemapStyle::Dataviz => format_tile_url("dataviz", use_scoria),
        BasemapStyle::Streets => format_tile_url("streets-v2", use_scoria),
        BasemapStyle::Topo => format_tile_url("topo-v2", use_scoria),
        BasemapStyle::Outdoor => format_tile_url("outdoor-v2", use_scoria),
        BasemapStyle::BasicDark => format_tile_url("basic-v2-dark", use_scoria),
        BasemapStyle::DatavizDark => {
            format_tile_url("dataviz-dark", use_scoria)
        }
        BasemapStyle::StreetsDark => {
            format_tile_url("streets-v2-dark", use_scoria)
        }
        BasemapStyle::TopoDark => format_tile_url("topo-v2-dark", use_scoria),
        BasemapStyle::OutdoorDark => {
            format_tile_url("outdoor-v2-dark", use_scoria)
        }
        BasemapStyle::Satellite => format_tile_url("hybrid", use_scoria),
    }
}

// YEW COMPONENT

#[function_component]
pub fn MapStyler() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let style = use_selector(|s: &FrontState| s.map.style.clone());

    let color_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.solid_color.rgb = val.to_string();
        },
    );
    let opacity_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: InputEvent| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_opacity = val.to_string().parse::<f64>().unwrap();
            s.map.style.solid_color.a = new_opacity;
        },
    );
    let marker_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: InputEvent| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.marker_size = val.to_string().parse().unwrap();
        },
    );
    let line_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: InputEvent| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.line_size = val.to_string().parse().unwrap();
        },
    );
    let basemap_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.basemap_style = BasemapStyle::from_str(val).unwrap();
        },
    );
    let datastream_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.colored_datastream =
                ColoredDataStream::from_str(val).unwrap();
        },
    );
    let colorbar_on_click = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.show_colorbar = !s.map.style.show_colorbar;
        })
    };

    let basemap_options = BASEMAP_STRINGS.iter().map(|x| {
        html! {
            <option selected={x.0 == style.basemap_style}>
                {x.1.to_string()}
            </option>
        }
    });
    let datastream_options = DATASTREAM_STRINGS.iter().map(|x| {
        html! {
            <option selected={x.0 == style.colored_datastream}>
                {x.1.to_string()}
            </option>
        }
    });

    html! {
        <div class="flex">
        <div class="max-w-fit mx-auto">
            <div class="flex items-center justify-between h-8 flex-wrap">
                <label for="opacity">{"Opacity"}</label>
                <input type="range" id="opacity"
                    value={format!("{}", style.solid_color.a)}
                    min="0.0" max="1.0" step="0.01"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    oninput={opacity_onchange} />
            </div>
            <div class="flex items-center justify-between h-8 flex-wrap">
                <label for="marker_size">{"Point Size"}</label>
                <input type="range" id="marker_size"
                    value={format!("{}", style.marker_size)}
                    min="0" max="10"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    oninput={marker_size_onchange} />
            </div>
            <div class="flex items-center justify-between h-8 flex-wrap">
                <label for="line_size">{"Line Size"}</label>
                <input type="range" id="line_size"
                    value={format!("{}", style.line_size)}
                    min="0" max="10"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    oninput={line_size_onchange} />
            </div>
            <div class="flex items-center justify-between flex-wrap">
                <label for="basemap">{"Basemap Style"}</label>
                <select onchange={basemap_onchange} id="basemap"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for basemap_options}
                </select>
            </div>
            <div class="flex items-center justify-between flex-wrap">
                <label for="datastream">{"Data Coloring"}</label>
                <select onchange={datastream_onchange} id="datastream"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for datastream_options}
                </select>
            </div>
            if style.colored_datastream == ColoredDataStream::None {
                <div class="flex items-center justify-between flex-wrap">
                    <label for="marker_color">{"Marker Color"}</label>
                    <input type="color" id="marker_color"
                        value={style.solid_color.rgb.clone()}
                        class="m-1 ml-6 bg-neutral-800"
                        onchange={color_onchange} />
                </div>
            } else if style.colored_datastream != ColoredDataStream::Time {
                // since no colorbar for time, colorbar option is hidden
                <div class="flex items-center justify-between flex-wrap">
                    <label for="colorbar">{"Colorbar"}</label>
                    <div class="relative ml-4 mr-1 h-6">
                        <input type="checkbox" id="colorbar"
                            checked={style.show_colorbar}
                            onclick={colorbar_on_click}
                            class={TOGGLE_SWITCH_STYLE} />
                    </div>
                </div>
            }
        </div>
        </div>
    }
}
