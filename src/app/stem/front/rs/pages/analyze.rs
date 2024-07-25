//! Analysis of collected data.

use std::rc::Rc;

use gloo_net::http::Request;
use serde_json::Value;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::pin_editor::PinDetails;
use crate::components::select_points_control::{
    handle_nearest_location, SelectPointsControl,
};
use crate::maplibre::{self, add_source_and_layers_to_style};
use crate::pages::metrics_dashboard::ColoredTimeSeriesPlot;
use crate::ui_state::{BackState, DerivedState, FrontState};
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};
use crate::{
    components::{
        location_filter_list::LocationFilterList,
        map_styler::{get_basemap_url, MapStyler},
        Colorbar, TabBar, TimeRangePicker, PRIMARY_BUTTON_STYLE,
        SECONDARY_BUTTON_STYLE,
    },
    maplibre::view_pos_from_map,
};
use common::state::MapSettingsTab;
use common::LngLat;

const BLANK_MAP_STYLE_DARK: &str = r#"{"version":8,"name":"Blank","sources":{},"layers":[{"id":"Background","type":"background","layout":{"visibility":"visible"},"paint":{"background-color":["interpolate",["exponential",1],["zoom"],6,"hsl(0, 0%, 17%)",20,"hsl(0, 0%, 18%)"]}}],"center":[0,0],"zoom":1}"#;
const BLANK_MAP_STYLE_LIGHT: &str = r#"{"version":8,"name":"Blank","sources":{},"layers":[{"id":"Background","type":"background","layout":{"visibility":"visible"},"paint":{"background-color":{"stops":[[6,"hsl(60,20%,85%)"],[20,"hsl(60,24%,90%)"]]}}}],"center":[0,0],"zoom":1}"#;

#[function_component]
pub fn Analyze() -> Html {
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

    let settings_tab =
        use_selector(|s: &FrontState| s.map.settings_tab.clone());
    // Get the number of filters clamped to the range [0, 2], which is where
    // resizing of the filter list occurs. If the value changes, trigger resize
    let num_filters = (*filters).len().clamp(0, 2);

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
            map_style.should_show_colorbar(),
        ),
    );

    html! {
        <div class="flex-1 flex flex-col">
            <PlotComponent />
            if *settings_tab == MapSettingsTab::MapStyle {
                <MapStyler />
            }
            if *settings_tab == MapSettingsTab::TimeRange {
                <TimeRangePicker />
            }
            if *settings_tab == MapSettingsTab::Filters {
                <LocationFilterList />
            }
            if *settings_tab == MapSettingsTab::PinDetails {
                <PinDetails />
            }
            if *settings_tab == MapSettingsTab::TimeSeriesPlot {
                <ColoredTimeSeriesPlot />
            }
            if *settings_tab == MapSettingsTab::SelectPoints {
                <SelectPointsControl />
            }
            <SettingsPicker />
        </div>
    }
}

#[function_component]
fn SettingsPicker() -> Html {
    let tab = use_selector(|s: &FrontState| s.map.settings_tab.clone());
    let front_dispatch = Dispatch::<FrontState>::new();
    // callback generic to all settings tabs
    let onclick = front_dispatch.reduce_mut_callback_with(
        |state: &mut FrontState, tab_target: MapSettingsTab| {
            if state.map.settings_tab == tab_target {
                // Already on this tab -> close it
                state.map.settings_tab = MapSettingsTab::None;
            } else {
                // Not on the tab yet -> go to it
                state.map.settings_tab = tab_target;
            }
        },
    );
    // };
    let time_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(MapSettingsTab::TimeRange);
        })
    };
    let style_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(MapSettingsTab::MapStyle);
        })
    };
    let filter_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(MapSettingsTab::Filters);
        })
    };
    let select_onclick = {
        let onclick = onclick.clone();
        Callback::from(move |_e: MouseEvent| {
            onclick.emit(MapSettingsTab::SelectPoints);
        })
    };
    let plot_onclick = Callback::from(move |_e: MouseEvent| {
        onclick.emit(MapSettingsTab::TimeSeriesPlot);
    });
    let get_style = move |tab_target: MapSettingsTab| {
        if *tab == tab_target {
            format!("m-1 p-2 {}", PRIMARY_BUTTON_STYLE)
        } else {
            format!("m-1 p-2 {}", SECONDARY_BUTTON_STYLE)
        }
    };
    let style_button_style = get_style(MapSettingsTab::MapStyle);
    let time_button_style = get_style(MapSettingsTab::TimeRange);
    let filter_button_style = get_style(MapSettingsTab::Filters);
    let plot_button_style = get_style(MapSettingsTab::TimeSeriesPlot);
    let select_button_style = get_style(MapSettingsTab::SelectPoints);
    html! {
        <div class="flex">
            <div class="mx-auto">
                <button onclick={select_onclick} id="select_points_btn"
                    class={select_button_style}>
                        <Icon icon_id={IconId::BootstrapHandIndexThumb}
                            class="h-6 w-6" />
                </button>
                <button onclick={plot_onclick} id="graph_btn"
                    class={plot_button_style}>
                        <Icon icon_id={IconId::BootstrapGraphUp}
                            class="h-6 w-6" />
                </button>
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
    let map = use_state(|| Option::<Rc<maplibre::binds::Map>>::None);

    // === Popup Callbacks === //

    // called by maplibre when a click is detected
    let click_point = {
        let front_dispatch = Dispatch::<FrontState>::new();
        let wss = use_context::<WebsocketService>().unwrap();
        move |params: (LngLat, Option<String>)| {
            wss.send_msg(ToBack::GetLocationNear(params.0));
            // save the color of the point that was clicked, so we can color
            // the background with it
            front_dispatch
                .reduce_mut(|s: &mut FrontState| s.map.popup_color = params.1);
        }
    };
    // adds the popup to the map when the popup contents come from the backend
    let on_get_nearest_location = {
        let map = map.clone();
        move |msg: &ToFront| {
            if let ToFront::NearestLocation(loc) = msg {
                if let Some(map) = (*map).clone() {
                    handle_nearest_location(map, loc);
                }
            }
        }
    };
    use_backend_event_with_deps(on_get_nearest_location, map.clone());

    // === Initial Map === //

    let plot_id = "map-div";
    let map_initialized = use_state(|| false);

    let map_style = use_selector(|s: &FrontState| s.map.style.clone());

    // The style json string for the basemap without user data on top
    let basemap = use_state(|| Option::<Value>::None);
    {
        let basemap = basemap.clone();
        use_effect_with_deps(
            move |basemap_style| {
                let basemap_style = *basemap_style;
                wasm_bindgen_futures::spawn_local(async move {
                    let blank_map = match basemap_style.is_dark() {
                        true => BLANK_MAP_STYLE_DARK,
                        false => BLANK_MAP_STYLE_LIGHT,
                    };
                    let basemap_str = match basemap_style.is_some() {
                        false => String::from(blank_map),
                        true => {
                            let url = get_basemap_url(&basemap_style);
                            let text = match Request::get(&url).send().await {
                                Ok(resp) => resp.text().await.ok(),
                                Err(_) => None,
                            };
                            text.unwrap_or_else(|| String::from(blank_map))
                        }
                    };
                    // if we can't parse the json, just make it a blank map
                    let basemap_obj: Value = serde_json::from_str(&basemap_str)
                        .unwrap_or_else(|_| {
                            serde_json::from_str(blank_map).unwrap()
                        });
                    basemap.set(Some(basemap_obj));
                });
                || ()
            },
            map_style.basemap_style,
        );
    }

    // The style object with user data
    let style = use_state(|| Option::<Value>::None);
    // update last location
    let last_loc =
        use_selector(|state: &BackState| state.last_location.clone());
    let last_loc_lnglat = last_loc.as_ref().as_ref().map(|x| x.lnglat());
    {
        let style = style.clone();
        use_effect_with_deps(
            move |(basemap, map_style)| {
                // if we've loaded the basemap json from the http request
                if let Some(basemap_obj) = &**basemap {
                    let mut style_obj = basemap_obj.clone();
                    add_source_and_layers_to_style(
                        &mut style_obj,
                        map_style,
                        last_loc_lnglat,
                    );
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
        let view_position = use_selector(|s: &FrontState| s.map.view_pos);
        let front_dispatch = Dispatch::<FrontState>::new();
        let pin_onclick = |maybe_id| {
            Dispatch::<FrontState>::new().reduce_mut(
                move |state: &mut FrontState| {
                    state.map.selected_pin_id = maybe_id;
                    state.map.settings_tab = MapSettingsTab::PinDetails;
                },
            )
        };
        let on_create_pin = |loc: LngLat| {
            Dispatch::<FrontState>::new().reduce_mut(
                move |state: &mut FrontState| {
                    state.map.selected_pin_id = None;
                    state.map.current_pin = Default::default();
                    state.map.current_pin.lnglat = loc;
                    state.map.settings_tab = MapSettingsTab::PinDetails;
                    state.map.editable_pin = true;
                },
            )
        };
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
                            click_point,
                        );
                        // register the callbacks for click/create pin
                        maplibre::pins::register_callbacks(
                            &newmap,
                            pin_onclick,
                            on_create_pin,
                        );
                        map.set(Some(newmap));
                    }
                }
            },
            style.clone(),
        )
    };
    // cleanup the map when we navigate away
    use_effect_with_deps(
        move |map| {
            let map = map.clone();
            move || {
                if let Some(map) = &*map {
                    map.remove();
                }
            }
        },
        map.clone(),
    );
    // on load, retrieve the view bounds and send the bounds to the backend
    {
        let front_dispatch = Dispatch::<FrontState>::new();
        let map = map.clone();
        use_effect_with_deps(
            move |map_initialized| {
                if **map_initialized {
                    let view_pos = view_pos_from_map(&(*map).clone().unwrap());
                    front_dispatch.reduce_mut(|s| s.map.view_pos = view_pos)
                }
            },
            map_initialized.clone(),
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

    {
        let map = map.clone();
        let show_last_location = map_style.show_last_location;
        use_effect_with_deps(
            move |(map_initialized, last_loc_lnglat)| {
                if **map_initialized && show_last_location {
                    maplibre::update_last_location(
                        (*map).clone().unwrap(),
                        *last_loc_lnglat,
                    );
                }
            },
            (map_initialized.clone(), last_loc_lnglat),
        )
    };

    // update the visible pins to include unsaved edits
    let pins = use_selector(|state: &DerivedState| state.pins.clone());
    let current_pin =
        use_selector(|state: &FrontState| state.map.current_pin.clone());
    let editing_pin = use_selector(|state: &FrontState| state.map.editable_pin);
    let visible_pins = use_state(Vec::<common::pin::Pin>::new);
    {
        let visible_pins = visible_pins.clone();
        use_effect_with_deps(
            move |(pins, current_pin, editing)| {
                visible_pins.set(maplibre::pins::get_visible_pins(
                    (**pins).clone(),
                    (**current_pin).clone(),
                    **editing,
                ));
            },
            (pins.clone(), current_pin.clone(), editing_pin),
        );
    }
    // update pins when the map initializes, or the pins change
    {
        let map = map.clone();
        use_effect_with_deps(
            move |(map_initialized, visible_pins)| {
                if **map_initialized {
                    maplibre::pins::update_pins(
                        &(*map).clone().unwrap(),
                        visible_pins,
                    )
                    .unwrap();
                }
            },
            (map_initialized.clone(), visible_pins.clone()),
        )
    };

    // update selected points when map initializes, or selections change
    let selected =
        use_selector(|state: &FrontState| state.selected_points.clone());
    {
        let map = map.clone();
        use_effect_with_deps(
            move |(map_initialized, selected)| {
                if **map_initialized {
                    maplibre::selected_points::update_selected(
                        &(*map).clone().unwrap(),
                        selected,
                    )
                    .unwrap();
                }
            },
            (map_initialized.clone(), selected.clone()),
        )
    };

    // === Restyle map === //
    {
        let map = map.clone();
        use_effect_with_deps(
            move |style| {
                if *map_initialized {
                    if let Some(style) = &**style {
                        maplibre::restyle((*map).clone().unwrap(), style);
                        // also update pins, since the style object clears the
                        // data
                        maplibre::pins::update_pins_after_restyle(
                            (*map).clone().unwrap(),
                            (*visible_pins).clone(),
                        );
                        maplibre::selected_points::update_after_restyle(
                            (*map).clone().unwrap(),
                            (*selected).clone(),
                        );
                    }
                }
            },
            style,
        )
    };

    // === Re-center Plot on Click === //

    let flytodata_onclick = {
        let map = map.clone();
        let data_center = use_selector(|state: &BackState| state.data_center);
        Callback::from(move |_e: MouseEvent| {
            if let Some(center) = &*data_center {
                maplibre::fly_to((*map).clone().unwrap(), center.0, center.1)
            }
        })
    };

    let flytome_onclick = Callback::from(move |_e: MouseEvent| {
        if let Some(loc) = last_loc_lnglat {
            maplibre::fly_to((*map).clone().unwrap(), loc, 16.);
        }
    });

    html! {
        <>
        <div id={plot_id} class="w-screen flex-1 min-h-0 relative z-0">
            <button onclick={flytodata_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[3.375rem] left-safe-or-2.5 z-40">
                <Icon icon_id={IconId::BootstrapFullscreen}
                    class="h-6 w-6 text-[#aaaaaa]" />
            </button>
            <button onclick={flytome_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[6.125rem] left-safe-or-2.5 z-40">
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
