use std::str::FromStr;

use uom::fmt::DisplayStyle;
use uom::si::information;
use uom::si::u64::*;
use uom::str::ParseQuantityError;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::TOGGLE_SWITCH_STYLE;
use crate::ui_state::{BackState, FrontState};

const MIN_CACHE_SIZE: u64 = 10_000_000;

#[function_component]
pub fn AutomapSetting() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let style = use_selector(|s: &FrontState| s.map.style.clone());

    let automap_onclick = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.automap = !s.map.style.automap;
        })
    };

    let num_remaining =
        use_selector(|state: &BackState| state.num_automap_records_remaining);
    let status_text = if *num_remaining < 5 {
        // "Status: up-to-date".into()
        "Status: Updating disabled until bug fix in next release".into()
    } else {
        format!("Status: {num_remaining} records remaining.")
    };

    html! {
        <>
            // settings card
            <div class="bg-neutral-900 rounded-lg px-4 mt-2">
                // settings line
                <div class="py-2 w-full flex items-center justify-between \
                    flex-wrap">
                    <label label="hide_unexplored">
                        {"Hide Unexplored Area"}
                    </label>
                    <div class="relative ml-4 mr-1 h-6">
                        <input type="checkbox" id="hide_unexplored"
                            checked={style.automap}
                            onclick={automap_onclick}
                            class={TOGGLE_SWITCH_STYLE} />
                    </div>
                </div>
            </div>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Leaves the map blank where you haven't explored yet. An
                area is marked as explored if it is within 100 meters of a data
                point that has a horizontal error better (smaller) than 100
                meters."}
            </p>
            if style.automap {
                <p class="text-neutral-500 text-left px-2 pt-1">
                    {status_text}
                </p>
            }
        </>
    }
}

fn format_bytes_as_megabytes(bytes: u64) -> String {
    let cache_size_bytes = Information::new::<information::byte>(bytes);
    Information::format_args(information::megabyte, DisplayStyle::Abbreviation)
        .with(cache_size_bytes)
        .to_string()
}

fn parse_information_to_bytes(val: &str) -> Result<u64, ParseQuantityError> {
    let result =
        Information::from_str(val).map(|x| x.get::<information::byte>());
    if result.is_err() {
        if let Ok(parsed) = val.parse::<f64>() {
            return Ok((parsed * 1_000_000.0).floor() as u64);
        }
    }
    result
}

#[function_component]
pub fn CacheSetting() -> Html {
    let dispatch = Dispatch::<FrontState>::new();

    let cache_pref = use_selector(|s: &FrontState| s.map_cache_pref.clone());
    let max_cache_size_text = format_bytes_as_megabytes(cache_pref.max_size);
    let max_cache_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(mut val) = parse_information_to_bytes(&elem.value()) {
                // ensure we don't go below the minimum cache size
                val = val.max(MIN_CACHE_SIZE);
                s.map_cache_pref.max_size = val;
            }
            elem.set_value("");
        },
    );

    // size of the cache right now
    let curr_cache_size = use_selector(|s: &BackState| s.map_cache_size);
    // let curr_cache_size_text = format!("Current cache size:

    let disable_fetch_onclick = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map_cache_pref.disable_fetch = !s.map_cache_pref.disable_fetch;
        })
    };

    html! {
        <>
            // settings card
            <div class="bg-neutral-900 rounded-lg px-4 mt-2">
                // settings line
                <div class="py-2 w-full flex items-center justify-between \
                    flex-wrap">
                    <label for="cache_size">{"Maximum Cache Size"}</label>
                    <input onchange={max_cache_size_onchange}
                        id="cache_size"
                        placeholder={max_cache_size_text}
                        class="ml-4 w-24 rounded bg-black \
                        border border-neutral-700 \
                        placeholder:text-neutral-500" />
                </div>
            </div>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Maximum amount of map data to cache on-device. Cached data
                does not need to be fetched from the network, allowing offline
                access and reduced data use. When the cache size exceeds the
                maximum, the least recently used data is deleted.
                100 MB or greater is recommended and 10 MB is the minimum."}
            </p>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Cached data older than seven days is refreshed from the
                network if the network is available. Map styles are not deleted
                once cached; only region-specific map data is deleted to keep
                the cache size within the limit."}
            </p>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Current cache size: "}
                {format_bytes_as_megabytes(*curr_cache_size)}
            </p>

            // settings card
            <div class="bg-neutral-900 rounded-lg px-4 mt-2">
                // settings line
                <div class="py-2 w-full flex items-center justify-between \
                    flex-wrap">
                    <label for="disable_fetch">
                        {"Disable Network Requests"}
                    </label>
                    <div class="relative ml-4 mr-1 h-6">
                        <input type="checkbox" id="disable_fetch"
                            checked={cache_pref.disable_fetch}
                            onclick={disable_fetch_onclick}
                            class={TOGGLE_SWITCH_STYLE} />
                    </div>
                </div>
            </div>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Prevents loading new map data over the internet, making all
                map data only come from the map cache. Use this to keep the
                map cache as-is or to prevent unwanted network data use.
                Map data not found in the cache will appear blank on the map."}
            </p>

        </>
    }
}
