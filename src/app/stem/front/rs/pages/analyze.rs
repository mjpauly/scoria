//! Analysis of collected data.

use std::rc::Rc;

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{
    datastream::DataStream,
    location_filter_list::{
        apply_filters, Filter, FilterOp, LocationFilterList,
    },
    map_styler::{
        use_check_epsln_tile_server, BasemapStyle, ColoredDataStream, MapStyle,
        MapStyler, Rgba,
    },
    time_range_picker::time_range_today,
    NavbarWrapper, TimeRangePicker, PRIMARY_BUTTON_STYLE,
    SECONDARY_BUTTON_STYLE,
};
use crate::plots::maplibre;
use crate::ui_state::UIState;
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};
use common::TimeRange;

#[derive(Debug, Clone, PartialEq)]
struct ViewDataPlaceholder();

#[function_component]
pub fn Analyze() -> Html {
    // Check if the epsln tile server is up, and update the app state
    use_check_epsln_tile_server();

    html! {
        <NavbarWrapper>
            <AnalyzeLocation />
        </NavbarWrapper>
    }
}

#[function_component]
fn AnalyzeLocation() -> Html {
    // Plot configuration values passed to children
    let time_range = use_state(time_range_today);
    let map_style = MapStyle {
        solid_color: use_state(|| Rgba {
            rgb: String::from("#0a84ff"),
            a: 1.0,
        }),
        marker_size: use_state(|| 3_usize),
        basemap_style: use_state(|| BasemapStyle::BasicDark),
        colored_datastream: use_state(|| ColoredDataStream::None),
    };
    let filters = use_state(|| {
        vec![Filter {
            id: 0,
            enabled: false,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 10.0,
        }]
    });
    let relayout_data = use_state(|| Option::<ViewDataPlaceholder>::None);

    // Ask for new location data from backend after render and do a full plot
    // reload anytime time_range changes
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        move |time_range| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        time_range.clone(),
    );

    let settings_tab = use_state(|| SettingsTab::None);
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
        (settings_tab.clone(), num_filters),
    );

    html! {
        <div class="flex flex-col h-full">
            <PlotComponent
                time_range={time_range.clone()}
                map_style={map_style.clone()}
                filters={filters.clone()}
                relayout_data={relayout_data.clone()}
            />
            if *settings_tab == SettingsTab::MapStyle {
                <MapStyler map_style={map_style.clone()} />
            }
            if *settings_tab == SettingsTab::TimeRange {
                <TimeRangePicker time_range={time_range.clone()} />
            }
            if *settings_tab == SettingsTab::Filters {
                <LocationFilterList filters={filters.clone()} />
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

/// Show a plot given a time range of values. Auto updates if new data streamed
/// from backend falls within the time range.
#[derive(Properties, PartialEq)]
struct PlotComponentProps {
    time_range: UseStateHandle<TimeRange>,
    map_style: MapStyle,
    filters: UseStateHandle<Vec<Filter>>,
    relayout_data: UseStateHandle<Option<ViewDataPlaceholder>>,
}

#[function_component]
fn PlotComponent(
    PlotComponentProps {
        time_range,
        map_style,
        filters,
        relayout_data: _,
    }: &PlotComponentProps,
) -> Html {
    let use_epsln_tile_server =
        use_selector(|s: &UIState| s.use_epsln_tile_server);
    // Show new plot if time range changes and new data comes from backend

    let plot_id = "map-div";
    let plot_initialized = use_state(|| false);
    let map = use_state(|| Option::<Rc<maplibre::Map>>::None);
    let records = use_state(|| Vec::<common::Location>::new());

    let on_backend_msg = {
        let map = map.clone();
        let records = records.clone();
        let plot_initialized = plot_initialized.clone(); // only exports values
        let map_style = map_style.clone();
        let filters = filters.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                // save records
                let mut newrecords = Vec::new();
                for rec in locations {
                    newrecords.push(rec.clone());
                }
                records.set(newrecords);

                // filter out unwanted data
                let locations = apply_filters(&filters, locations);

                // set the guard to prevent map updates until it has loaded
                plot_initialized.set(false);
                let plot_initialized = plot_initialized.clone();
                let on_load = {
                    let plot_initialized = plot_initialized.clone();
                    Box::new(move || plot_initialized.set(true))
                };
                let newmap = maplibre::new_map(
                    plot_id,
                    &locations,
                    map_style.basemap_style.get_url(*use_epsln_tile_server),
                    *map_style.marker_size,
                    map_style.solid_color.rgb.clone(),
                    map_style.solid_color.a,
                    &map_style.colored_datastream,
                    on_load,
                );
                map.set(Some(newmap));
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (map_style.clone(), filters.clone()),
    );

    // Update plot with new data points without creating a new plot

    let on_backend_msg = {
        let map = map.clone();
        let records = records.clone();
        let time_range = time_range.clone();
        let map_style = map_style.clone();
        let filters = filters.clone();
        let plot_initialized = plot_initialized.clone();
        move |msg: &ToFront| {
            // if new location data streamed in
            if let ToFront::LastLocation(location) = msg {
                // if time range of data we're displaying contains the new data
                if time_range.contains(&location.datetime) {
                    let mut newrecords = (*records).clone();
                    newrecords.push(location.clone());
                    records.set(newrecords);
                    if *plot_initialized {
                        let recs = apply_filters(&filters, &*records);
                        maplibre::update_data(
                            (*map).clone().unwrap(),
                            &recs,
                            &map_style.colored_datastream,
                        );
                    }
                }
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (
            map.clone(),
            records.clone(),
            time_range.clone(),
            map_style.clone(),
            filters.clone(),
            plot_initialized.clone(),
        ),
    );

    // Restyle hook (map style or filters updated)
    {
        let map = map.clone();
        let records = records.clone();
        let plot_initialized = plot_initialized.clone();
        use_effect_with_deps(
            move |(map_style, filters)| {
                if *plot_initialized {
                    let recs = apply_filters(&filters, &*records);
                    // might need to update color property of data source
                    maplibre::update_data(
                        (*map).clone().unwrap(),
                        &recs,
                        &map_style.colored_datastream,
                    );
                    // restyle the map layer that shows the source
                    maplibre::restyle_layer(
                        (*map).clone().unwrap(),
                        *map_style.marker_size,
                        map_style.solid_color.rgb.clone(),
                        map_style.solid_color.a,
                        &map_style.colored_datastream,
                    );
                }
            },
            (map_style.clone(), filters.clone()),
        )
    };

    html! {
        <div id={plot_id} class="w-screen flex-1 min-h-0">
        </div>
    }
}
