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

/// Hook for checking if the epsln tile server is up. If it is, we update the
/// frontend state to get tiles from it.
#[hook]
pub fn use_check_epsln_tile_server() {
    let dispatch = Dispatch::<FrontState>::new();
    yew::platform::spawn_local(async move {
        let result = Request::get(obfstr!("https://api.epsln.com/maps_ok"))
            .send()
            .await;
        if let Ok(resp) = result {
            dispatch.reduce_mut(|s: &mut FrontState| {
                s.use_epsln_tile_server = resp.status() == 200
            });
        } else {
            dispatch.reduce_mut(|s: &mut FrontState| {
                s.use_epsln_tile_server = false
            });
        }
    });
}

fn format_tile_url(style: &str, use_epsln: bool) -> String {
    // get the secrets in the .env file at compile time and obfuscate them
    obfstr! {
        let maptiler_key = dotenvy_macro::dotenv!(
            "MAPTILER_API_KEY",
            "Maptiler API key must be placed in top-level .env file as \
            MAPTILER_API_KEY={key}"
        );
        let epsln_key = dotenvy_macro::dotenv!(
            "EPSLN_TILE_API_KEY",
            "Epsilon API key must be placed in top-level .env file as \
            EPSLN_TILE_API_KEY={key}"
        );
        let maptiler_base_url = "https://api.maptiler.com/maps";
        let epsln_base_url = "https://api.epsln.com/maps";
        let style_json = "style.json?key=";
    }
    let is_satellite = style.contains("hybrid");
    // still use maptiler for satellite images, even if use_epsln is true
    let (base_url, key) = if use_epsln && !is_satellite {
        (epsln_base_url, epsln_key)
    } else {
        (maptiler_base_url, maptiler_key)
    };
    // should be https://api.url.com/maps/basic-v2/style.json?key=deadbeef
    format!("{base_url}/{style}/{style_json}{key}")
}

pub fn get_basemap_url(style: &BasemapStyle, use_epsln: bool) -> String {
    match style {
        BasemapStyle::Basic => format_tile_url("basic-v2", use_epsln),
        BasemapStyle::Dataviz => format_tile_url("dataviz", use_epsln),
        BasemapStyle::Streets => format_tile_url("streets-v2", use_epsln),
        BasemapStyle::Topo => format_tile_url("topo-v2", use_epsln),
        BasemapStyle::Outdoor => format_tile_url("outdoor-v2", use_epsln),
        BasemapStyle::BasicDark => format_tile_url("basic-v2-dark", use_epsln),
        BasemapStyle::DatavizDark => format_tile_url("dataviz-dark", use_epsln),
        BasemapStyle::StreetsDark => {
            format_tile_url("streets-v2-dark", use_epsln)
        }
        BasemapStyle::TopoDark => format_tile_url("topo-v2-dark", use_epsln),
        BasemapStyle::OutdoorDark => {
            format_tile_url("outdoor-v2-dark", use_epsln)
        }
        BasemapStyle::Satellite => format_tile_url("hybrid", use_epsln),
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
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_opacity = val.to_string().parse::<f64>().unwrap();
            s.map.style.solid_color.a = new_opacity;
        },
    );
    let marker_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            s.map.style.marker_size = val.to_string().parse().unwrap();
        },
    );
    let line_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
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
        <div class="mt-2 mb-1 flex">
        <div class="max-w-fit mx-auto">
            <div class="flex items-center justify-between h-8">
                <label for="opacity">{"Marker Opacity"}</label>
                <input type="range" id="opacity"
                    value={format!("{}", style.solid_color.a)}
                    min="0.0" max="1.0" step="0.01"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    onchange={opacity_onchange} />
            </div>
            <div class="flex items-center justify-between h-8">
                <label for="marker_size">{"Circle Radius"}</label>
                <input type="range" id="marker_size"
                    value={format!("{}", style.marker_size)}
                    min="0" max="10"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    onchange={marker_size_onchange} />
            </div>
            <div class="flex items-center justify-between h-8">
                <label for="line_size">{"Line Width"}</label>
                <input type="range" id="line_size"
                    value={format!("{}", style.line_size)}
                    min="0" max="10"
                    class={format!("m-1 ml-6 {}", RANGE_INPUT_STYLE)}
                    onchange={line_size_onchange} />
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
            if style.colored_datastream == ColoredDataStream::None {
                <div class="flex items-center justify-between">
                    <label for="marker_color">{"Marker Color"}</label>
                    <input type="color" id="marker_color"
                        value={style.solid_color.rgb.clone()}
                        class="m-1 ml-6 bg-neutral-800"
                        onchange={color_onchange} />
                </div>
            } else if style.colored_datastream != ColoredDataStream::Time {
                // since no colorbar for time, colorbar option is hidden
                <div class="flex items-center justify-between">
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
