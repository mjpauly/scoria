use std::str::FromStr;

use uom::fmt::DisplayStyle;
use uom::si::information;
use uom::si::u64::*;
use uom::str::ParseQuantityError;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_icons::Icon;
use yew_icons::IconId;
use yewdux::prelude::*;

use crate::components::RANGE_INPUT_STYLE;
use crate::components::{
    AfterCardParagraph, SettingsCard, SettingsCardInput, SettingsCardToggle,
};
use crate::ui_state::{BackState, FrontState};

const MIN_CACHE_SIZE: u64 = 10_000_000;

#[function_component]
pub fn MiscMapSettings() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let map = use_selector(|s: &FrontState| s.map.clone());

    let show_last_location_on_click =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.show_last_location = !s.map.style.show_last_location;
        });
    let pins_below_data_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.pins_below_data = !s.map.style.pins_below_data;
        });
    let open_in_google_maps_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.open_in_google_maps = !s.map.open_in_google_maps;
        });
    let hide_points_outside_viewbounds_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.hide_points_outside_viewbounds =
                !s.map.style.hide_points_outside_viewbounds;
        });

    html! {
        <>
            <SettingsCard>
                <SettingsCardToggle
                    checked={map.style.show_last_location}
                    onclick={show_last_location_on_click}
                    text={"Show Last Location"}
                />
                <SettingsCardToggle
                    checked={map.style.pins_below_data}
                    onclick={pins_below_data_onclick}
                    text={"Display Pins Below Log Data"}
                />
                if cfg!(feature = "ios_config") {
                    <SettingsCardToggle
                        checked={map.open_in_google_maps}
                        onclick={open_in_google_maps_onclick}
                        text={"Open Links in Google Maps"}
                    />
                }
                <SettingsCardToggle
                    checked={map.style.hide_points_outside_viewbounds}
                    onclick={hide_points_outside_viewbounds_onclick}
                    text={"Hide Points Outside Viewbounds"}
                />
            </SettingsCard>
        </>
    }
}

#[function_component]
pub fn AutomapSetting() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let style = use_selector(|s: &FrontState| s.map.style.clone());

    let automap_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.style.automap = !s.map.style.automap;
        });
    let opacity_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: InputEvent| -> Option<()> {
            let elem: HtmlInputElement = e.target_dyn_into()?;
            let val: &str = &elem.value();
            let new_opacity = val.to_string().parse::<f64>().ok()?;
            s.map.style.automap_opacity = new_opacity;
            None
        },
    );
    let zero_opacity_icon_color = if style.automap_opacity == 0.0 {
        "text-primary"
    } else {
        "test-neutral-500"
    };
    let one_opacity_icon_color = if style.automap_opacity == 1.0 {
        "text-primary"
    } else {
        "test-neutral-500"
    };

    let num_remaining =
        use_selector(|state: &BackState| state.num_automap_records_remaining);
    let status_text = if *num_remaining < 5 {
        "Status: up-to-date".into()
    } else {
        format!("Status: {num_remaining} records remaining.")
    };

    html! {
        <>
            <SettingsCard>
                <SettingsCardToggle
                    checked={style.automap}
                    onclick={automap_onclick}
                    text={"Use Automap"}
                />
                <div class="flex items-center justify-between py-2 px-4 w-full \
                    active:bg-neutral-800">
                    <label for="opacity">{"Opacity"}</label>
                    <div class="flex items-center gap-2">
                        <Icon icon_id={IconId::BootstrapSquare}
                            class={classes!(
                                "h-4 w-4".to_string(),
                                zero_opacity_icon_color
                            )} />
                        <input type="range" id="opacity"
                            value={format!("{}", style.automap_opacity)}
                            min="0.0" max="1.0" step="0.01"
                            class={format!("m-1 {}", RANGE_INPUT_STYLE)}
                            oninput={opacity_onchange} />
                        <Icon icon_id={IconId::BootstrapSquareFill}
                            class={classes!(
                                "h-4 w-4".to_string(),
                                one_opacity_icon_color,
                            )} />
                    </div>
                </div>
            </SettingsCard>
            <AfterCardParagraph>
                {"The automap obscures the parts of the map where you haven't
                explored yet. An area is marked as explored if it is within 100
                meters of a data point that has a horizontal error better
                (smaller) than 100 meters."}
            </AfterCardParagraph>
            if style.automap {
                <AfterCardParagraph>
                    {status_text}
                </AfterCardParagraph>
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
        // Assume MB if it's just a number and retry parsing
        if let Ok(parsed) = val.parse::<f64>() {
            return Ok((parsed * 1_000_000.0).floor() as u64);
        }
    }
    result
}

#[function_component]
pub fn CacheSetting() -> Html {
    let dispatch = Dispatch::<FrontState>::new();

    let cache_pref = use_selector(|s: &FrontState| s.map_cache_pref);
    let max_cache_size_text = format_bytes_as_megabytes(cache_pref.max_size);
    let max_cache_size_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, value: String| {
            if let Ok(mut val) = parse_information_to_bytes(&value) {
                // ensure we don't go below the minimum cache size
                val = val.max(MIN_CACHE_SIZE);
                s.map_cache_pref.max_size = val;
            }
        },
    );

    // size of the cache right now
    let curr_cache_size = use_selector(|s: &BackState| s.map_cache_size);

    let disable_fetch_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map_cache_pref.disable_fetch = !s.map_cache_pref.disable_fetch;
        });

    html! {
        <>
            <SettingsCard>
                <SettingsCardInput
                    text="Maximum Cache Size"
                    value={max_cache_size_text}
                    onchange={max_cache_size_onchange}
                    class="w-24"
                />
            </SettingsCard>
            <AfterCardParagraph>
                {"Maximum amount of map data to cache on-device. Cached data
                does not need to be fetched from the network, allowing offline
                access and reduced data use. When the cache size exceeds the
                maximum, the least recently used data is deleted.
                200 MB or greater is recommended and 10 MB is the minimum."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"Cached data older than seven days is refreshed from the
                network if available."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"Current cache size: "}
                {format_bytes_as_megabytes(*curr_cache_size)}
            </AfterCardParagraph>

            <SettingsCard class="mt-4">
                <SettingsCardToggle
                    text="Disable Network Requests"
                    checked={cache_pref.disable_fetch}
                    onclick={disable_fetch_onclick}
                />
            </SettingsCard>
            <AfterCardParagraph>
                {"Prevents loading new map data over the internet, making all
                map data only come from the map cache. Use this to keep the
                map cache as-is or to prevent unwanted network data use.
                Map data not found in the cache will appear blank on the map."}
            </AfterCardParagraph>

        </>
    }
}
