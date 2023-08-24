//! Analysis of collected data.

use std::rc::Rc;

use gloo_net::http::Request;
use serde_json::Value;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{
    location_filter_list::LocationFilterList,
    map_styler::{get_basemap_url, use_check_scoria_tile_server, MapStyler},
    Colorbar, TabBar, TimeRangePicker, PRIMARY_BUTTON_STYLE,
    SECONDARY_BUTTON_STYLE,
};
use crate::plots::maplibre::{self, add_source_and_layers_to_style};
use crate::ui_state::{BackState, FrontState};
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};
use common::map_style::ColoredDataStream;
use common::LngLat;

#[function_component]
pub fn Analyze() -> Html {
    // Check if the scoria tile server is up, and update the app state
    use_check_scoria_tile_server();

    html! {
        <>
            <AnalyzeLocation />
            <TabBar />
        </>
    }
}

#[function_component]
fn AnalyzeLocation() -> Html {
    // Plot configuration values passed to children
    let map_style = use_selector(|s: &FrontState| s.map.style.clone());
    let filters = use_selector(|s: &FrontState| s.map.filters.clone());

    let settings_tab = use_state(|| SettingsTab::None);
    // Get the number of filters clamped to the range [0, 2], which is where
    // resizing of the filter list occurs. If the value changes, trigger resize
    let num_filters = (*filters).len().clamp(0, 2);
    // Colorbar toggle hidden on Time selection
    let colored_datastream_is_time =
        map_style.colored_datastream == ColoredDataStream::Time;

    // after rerender, trigger the plot's resize handler if the visible settings
    // tab has changed, or if that settings tab's size has changed
    use_effect_with_deps(
        move |_| {
            let event = web_sys::Event::new("resize").unwrap();
            web_sys::window().unwrap().dispatch_event(&event).unwrap();
        },
        (
            settings_tab.clone(),
            num_filters,
            colored_datastream_is_time,
            map_style.should_show_colorbar(),
        ),
    );

    html! {
        <div class="flex-1 flex flex-col">
            <PlotComponent />
            if *settings_tab == SettingsTab::MapStyle {
                <MapStyler />
            }
            if *settings_tab == SettingsTab::TimeRange {
                <TimeRangePicker />
            }
            if *settings_tab == SettingsTab::Filters {
                <LocationFilterList />
            }
            <SettingsPicker tab={settings_tab.clone()} />
        </div>
    }
}

#[derive(Clone, PartialEq, Debug)]
enum SettingsTab {
    None,
    Filters,
    MapStyle,
    TimeRange,
}

#[derive(Properties, PartialEq)]
struct SettingsPickerProps {
    tab: UseStateHandle<SettingsTab>,
}

#[function_component]
fn SettingsPicker(SettingsPickerProps { tab }: &SettingsPickerProps) -> Html {
    // callback generic to all settings tabs
    let onclick = {
        let tab = tab.clone();
        Callback::from(move |tab_target: SettingsTab| {
            if *tab == tab_target {
                // Already on this tab -> close it
                tab.set(SettingsTab::None);
            } else {
                // Not on the tab yet -> go to it
                tab.set(tab_target);
            }
        })
    };
    let time_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(SettingsTab::TimeRange);
        })
    };
    let style_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(SettingsTab::MapStyle);
        })
    };
    let filter_onclick = Callback::from(move |_e: MouseEvent| {
        onclick.emit(SettingsTab::Filters);
    });
    let get_style = move |tab_target: SettingsTab| {
        if **tab == tab_target {
            format!("m-1 p-2 {}", PRIMARY_BUTTON_STYLE)
        } else {
            format!("m-1 p-2 {}", SECONDARY_BUTTON_STYLE)
        }
    };
    let style_button_style = get_style(SettingsTab::MapStyle);
    let time_button_style = get_style(SettingsTab::TimeRange);
    let filter_button_style = get_style(SettingsTab::Filters);
    html! {
        <div class="flex">
            <div class="mx-auto">
                <button onclick={filter_onclick} id="filter_list_btn"
                    class={filter_button_style}>
                        <Icon icon_id={IconId::BootstrapFunnel}
                            class="h-6 w-6" />
                </button>
                <button onclick={style_onclick} id="map_style_btn"
                    class={style_button_style}>
                        <Icon icon_id={IconId::BootstrapBrush}
                            class="h-6 w-6" />
                </button>
                <button onclick={time_onclick} id="time_range_btn"
                    class={time_button_style}>
                        <Icon icon_id={IconId::BootstrapCalendarRange}
                            class="h-6 w-6" />
                </button>
            </div>
        </div>
    }
}

#[function_component]
fn PlotComponent() -> Html {
    // maplibre map handle
    let map = use_state(|| Option::<Rc<maplibre::Map>>::None);

    // === Popup Callbacks === //

    // called by maplibre when a click is detected
    let request_popup = {
        let wss = use_context::<WebsocketService>().unwrap();
        move |params: (LngLat, Option<String>)| {
            wss.send_msg(ToBack::GetPopupText(params));
        }
    };
    // adds the popup to the map when the popup contents come from the backend
    let on_get_popup_text = {
        let map = map.clone();
        move |msg: &ToFront| {
            // if *msg == ToFront::GeojsonUpdated && *map_initialized {
            if let ToFront::PopupText {
                location,
                text,
                bg_color,
            } = msg
            {
                // destructure Option just in case message is delivered at a
                // strange time
                if let Some(map) = (*map).clone() {
                    maplibre::add_popup(map, location, text, bg_color);
                }
            }
        }
    };
    use_backend_event_with_deps(on_get_popup_text, map.clone());

    // === Initial Map === //

    let plot_id = "map-div";
    let map_initialized = use_state(|| false);

    let map_style = use_selector(|s: &FrontState| s.map.style.clone());
    let use_scoria_tile_server =
        use_selector(|s: &FrontState| s.use_scoria_tile_server);

    // The style json string for the basemap without user data on top
    let basemap = use_state(|| Option::<Value>::None);
    {
        let basemap = basemap.clone();
        use_effect_with_deps(
            move |basemap_style| {
                let basemap_style = basemap_style.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let blank_map = String::from(
                        r#"{"version":8,"name":"Blank","sources":{},"layers":[],"center":[0,0],"zoom":1}"#,
                    );
                    let url = get_basemap_url(
                        &basemap_style,
                        *use_scoria_tile_server,
                    );
                    let basemap_str = match Request::get(&url).send().await {
                        Ok(req) => req
                            .text()
                            .await
                            .unwrap_or_else(|_| blank_map.clone()),
                        Err(_) => blank_map.clone(),
                    };
                    // if we can't parse the json, just make it a blank map
                    let basemap_obj: Value = serde_json::from_str(&basemap_str)
                        .unwrap_or_else(|_| {
                            serde_json::from_str(&blank_map).unwrap()
                        });
                    basemap.set(Some(basemap_obj));
                });
                || ()
            },
            map_style.basemap_style.clone(),
        );
    }

    // The style object with user data
    let style = use_state(|| Option::<Value>::None);
    {
        let style = style.clone();
        use_effect_with_deps(
            move |(basemap, map_style)| {
                // if we've loaded the basemap json from the http request
                if let Some(basemap_obj) = &**basemap {
                    let mut style_obj = basemap_obj.clone();
                    add_source_and_layers_to_style(&mut style_obj, map_style);
                    style.set(Some(style_obj));
                }
                || ()
            },
            (basemap, map_style.clone()),
        );
    }

    // Build the blank map on first render. We don't populate the map with any
    // data, but we do set up the source and layer needed to update the map.
    {
        let map = map.clone();
        let map_initialized = map_initialized.clone();
        let view_position =
            use_selector(|s: &FrontState| s.map.view_pos.clone());
        let front_dispatch = Dispatch::<FrontState>::new();
        use_effect_with_deps(
            move |style| {
                // only create the map when it's not created yet
                if !(*map_initialized) {
                    if let Some(style) = &**style {
                        let on_load = {
                            let map_initialized = map_initialized.clone();
                            Box::new(move || map_initialized.set(true))
                        };
                        let on_view_change = Box::new(move |data| {
                            front_dispatch.reduce_mut(|s| s.map.view_pos = data)
                        });
                        let newmap = maplibre::new_map(
                            plot_id,
                            style,
                            &view_position,
                            on_load,
                            on_view_change,
                            request_popup,
                        );
                        map.set(Some(newmap));
                    }
                }
            },
            style.clone(),
        )
    };

    // === Update map on new data === //

    // Update when the backend tells us the geojson is up-to-date
    let on_geojson_update = {
        let map = map.clone();
        let map_initialized = map_initialized.clone();
        move |msg: &ToFront| {
            if *msg == ToFront::GeojsonUpdated && *map_initialized {
                // OK to unwrap map when guarded by if *map_initialized, since
                // this is a contract we uphold
                maplibre::update_data((*map).clone().unwrap());
            }
        }
    };
    use_backend_event_with_deps(on_geojson_update, map_initialized.clone());

    // === Restyle map === //
    {
        let map = map.clone();
        use_effect_with_deps(
            move |style| {
                if *map_initialized {
                    if let Some(style) = &**style {
                        maplibre::restyle((*map).clone().unwrap(), style)
                    }
                }
            },
            style,
        )
    };

    // === Re-center Plot on Click === //

    let flytodata_onclick = {
        let map = map.clone();
        let data_center =
            use_selector(|state: &BackState| state.data_center.clone());
        Callback::from(move |_e: MouseEvent| {
            if let Some(center) = &*data_center {
                maplibre::fly_to(
                    (*map).clone().unwrap(),
                    (center.0.lng, center.0.lat),
                    center.1,
                )
            }
        })
    };

    let last_loc =
        use_selector(|state: &BackState| state.last_location.clone());
    let flytome_onclick = Callback::from(move |_e: MouseEvent| {
        if let Some(loc) = &*last_loc {
            maplibre::fly_to(
                (*map).clone().unwrap(),
                (loc.longitude, loc.latitude),
                16.,
            );
        }
    });

    html! {
        <>
        <div id={plot_id} class="w-screen flex-1 min-h-0 relative z-0">
            <button onclick={flytodata_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[3.375rem] left-2.5 z-40">
                <Icon icon_id={IconId::BootstrapFullscreen}
                    class="h-6 w-6 text-[#aaaaaa]" />
            </button>
            <button onclick={flytome_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[6.125rem] left-2.5 z-40">
                <Icon icon_id={IconId::FontAwesomeSolidLocationArrow}
                    class="h-6 w-6 text-[#aaaaaa]" />
            </button>
        </div>
        if map_style.should_show_colorbar() {
            <Colorbar />
        }
        </>
    }
}
