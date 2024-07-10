//! Picker element for marker color and opacity, basemap style

use std::str::FromStr;
use std::u8;

use obfstr::obfstr;
use strum::IntoEnumIterator;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{
    SettingsCard, RANGE_INPUT_STYLE, SELECT_STYLE, TOGGLE_SWITCH_STYLE,
};
use crate::router::get_scoped_host;
use crate::ui_state::FrontState;
use common::map_style::{
    BasemapStyle, ColoredDataStream, MARKER_SIZE_MAX, MARKER_SIZE_MIN,
};

// BASEMAP STYLES

fn format_tile_url(style: &str) -> String {
    // get the secrets in the .env file at compile time and obfuscate them
    obfstr! {
        let scheme = "http://";
        let basemap_path = "mapdata/maps";
        let style_json = "style.json";
    }
    let basemap_url = format!("{scheme}{}/{basemap_path}", get_scoped_host());
    // should be https://127.0.0.1/mapdata/maps/basic-v2/style.json
    format!("{basemap_url}/{style}/{style_json}")
}

pub fn get_basemap_url(style: &BasemapStyle) -> String {
    match style {
        BasemapStyle::Basic => format_tile_url("basic-v2"),
        BasemapStyle::Dataviz => format_tile_url("dataviz"),
        BasemapStyle::Streets => format_tile_url("streets-v2"),
        BasemapStyle::Topo => format_tile_url("topo-v2"),
        BasemapStyle::Outdoor => format_tile_url("outdoor-v2"),
        BasemapStyle::BasicDark => format_tile_url("basic-v2-dark"),
        BasemapStyle::DatavizDark => format_tile_url("dataviz-dark"),
        BasemapStyle::StreetsDark => format_tile_url("streets-v2-dark"),
        BasemapStyle::TopoDark => format_tile_url("topo-v2-dark"),
        BasemapStyle::OutdoorDark => format_tile_url("outdoor-v2-dark"),
        BasemapStyle::Hybrid => format_tile_url("hybrid"),
        BasemapStyle::Satellite => format_tile_url("satellite"),
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
    let basemap_options = BasemapStyle::iter().map(|x| {
        html! {
            <option selected={x == style.basemap_style}>
                {x.to_string()}
            </option>
        }
    });
    let datastream_options = ColoredDataStream::iter().map(|x| {
        html! {
            <option selected={x == style.colored_datastream}>
                {x.to_string()}
            </option>
        }
    });

    html! {
        <div class="flex m-1">
        <SettingsCard class="flex-grow mx-auto max-w-prose px-4 py-1">
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
                    min={MARKER_SIZE_MIN.to_string()}
                    max={MARKER_SIZE_MAX.to_string()}
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
                    <div class="relative my-1 ml-4 mr-1 h-6">
                        <input type="checkbox" id="colorbar"
                            checked={style.show_colorbar}
                            onclick={colorbar_on_click}
                            class={TOGGLE_SWITCH_STYLE} />
                    </div>
                </div>
            }
            <div class="flex items-center justify-between flex-wrap">
                <label for="datastream">{"Data Coloring"}</label>
                <select onchange={datastream_onchange} id="datastream"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for datastream_options}
                </select>
            </div>
            <div class="flex items-center justify-between flex-wrap">
                <label for="basemap">{"Basemap Style"}</label>
                <select onchange={basemap_onchange} id="basemap"
                    class={format!("m-1 ml-6 {}", SELECT_STYLE)}>
                    {for basemap_options}
                </select>
            </div>
        </SettingsCard>
        </div>
    }
}
