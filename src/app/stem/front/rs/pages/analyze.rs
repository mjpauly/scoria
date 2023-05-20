//! Analysis of collected data.

use std::{cell::RefCell, rc::Rc};

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
    Colorbar, NavbarWrapper, TimeRangePicker, PRIMARY_BUTTON_STYLE,
    SECONDARY_BUTTON_STYLE,
};
use crate::plots::maplibre;
use crate::plots::maplibre::ViewPosition;
use crate::ui_state::UIState;
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};
use common::TimeRange;

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
        line_size: use_state(|| 2_usize),
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
    let view_position = use_state(ViewPosition::default);

    let settings_tab = use_state(|| SettingsTab::None);
    // Get the number of filters clamped to the range [0, 2], which is where
    // resizing of the filter list occurs. If the value changes, trigger resize
    let num_filters = (*filters).len().clamp(0, 2);
    // Also resize if whether a colorbar is showing changes
    let colored_datastream_is_some = map_style.colored_datastream.is_some();
    // Time is also a special case for now
    let colored_datastream_is_time =
        *map_style.colored_datastream == ColoredDataStream::Time;

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
            colored_datastream_is_some,
            colored_datastream_is_time,
        ),
    );

    html! {
        <div class="flex flex-col h-full">
            <PlotComponent
                time_range={time_range.clone()}
                map_style={map_style.clone()}
                filters={filters.clone()}
                view_position={view_position.clone()}
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
    view_position: UseStateHandle<ViewPosition>,
}

#[function_component]
fn PlotComponent(
    PlotComponentProps {
        time_range,
        map_style,
        filters,
        view_position,
    }: &PlotComponentProps,
) -> Html {
    // === Cache location data === //

    // Ask for new location data from backend anytime time_range changes
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        move |time_range| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        time_range.clone(),
    );

    // State handle for the records to plot (but not yet filtered)
    let records = use_state(Vec::<common::Location>::new);

    // Save the data to plot when we receive it from the backend
    let on_backend_msg = {
        let records = records.clone();
        let time_range = time_range.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                let mut newrecords = Vec::new();
                for rec in locations {
                    newrecords.push(rec.clone());
                }
                records.set(newrecords);
            } else if let ToFront::LastLocation(location) = msg {
                if time_range.contains(&location.datetime) {
                    let mut newrecords = (*records).clone();
                    newrecords.push(location.clone());
                    records.set(newrecords);
                }
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (records.clone(), time_range.clone()),
    );

    // === Popup Display Callback === //

    // use_mut_ref lets us get up-to-date state values
    let filtered_records = use_mut_ref(Vec::<common::Location>::new);
    {
        let filtered_records = filtered_records.clone();
        use_effect_with_deps(
            move |(records, filters)| {
                let recs = apply_filters(filters, records);
                let mut newrecs = vec![];
                for r in recs {
                    newrecs.push(r.clone())
                }
                *filtered_records.borrow_mut() = newrecs;
            },
            (records.clone(), filters.clone()),
        );
    }
    let solid_color = use_mut_ref(|| map_style.solid_color.rgb.clone());
    {
        let solid_color = solid_color.clone();
        use_effect_with_deps(
            move |rgb| {
                *solid_color.borrow_mut() = rgb.clone();
            },
            map_style.solid_color.rgb.clone(),
        );
    }
    let get_popup_text = {
        // let filtered_records = filtered_records.clone();
        move |lng: f64, lat: f64, color: Option<String>| {
            let filtered_records = filtered_records.clone();
            let (lnglat, text) = get_hovertext(filtered_records, lng, lat);
            if let Some(data_color) = color {
                // return the location of the data point, the text to display
                // and the popup's background color
                (lnglat, text, data_color)
            } else {
                (lnglat, text, solid_color.borrow().clone())
            }
        }
    };

    // === Initial Map === //

    let use_epsln_tile_server =
        use_selector(|s: &UIState| s.use_epsln_tile_server);

    let plot_id = "map-div";
    let map_initialized = use_state(|| false);
    let map = use_state(|| Option::<Rc<maplibre::Map>>::None);
    let basemap = map_style.basemap_style.get_url(*use_epsln_tile_server);

    // Build the blank map on first render. We don't populate the map with any
    // data, but we do set up the source and layer needed to update the map.
    {
        let map = map.clone();
        let map_initialized = map_initialized.clone();
        let map_style = map_style.clone();
        let basemap = basemap.clone();
        let view_position = view_position.clone();
        use_effect_with_deps(
            move |_| {
                let on_load = {
                    let map_initialized = map_initialized.clone();
                    Box::new(move || map_initialized.set(true))
                };
                let on_view_change = {
                    let view_position = view_position.clone();
                    Box::new(move |data| view_position.set(data))
                };
                let newmap = maplibre::new_map(
                    plot_id,
                    &basemap,
                    *map_style.marker_size,
                    *map_style.line_size,
                    &map_style.solid_color,
                    &map_style.colored_datastream,
                    &view_position,
                    on_load,
                    on_view_change,
                    get_popup_text,
                );
                map.set(Some(newmap));
            },
            (),
        )
    };

    // === Update map on new data === //

    // Depends on records, filters, and map_initialized. The first two indicate
    // when the data shown needs to be updated. The third indicates if the map
    // just finished initializing, which likely happens after we already have
    // data to plot.
    {
        let map = map.clone();
        let map_style = map_style.clone();
        use_effect_with_deps(
            move |(records, filters, map_initialized): &(
                UseStateHandle<Vec<common::Location>>,
                UseStateHandle<Vec<Filter>>,
                UseStateHandle<bool>,
            )| {
                if **map_initialized {
                    let map = map.clone();
                    let records = records.clone();
                    let filters = filters.clone();
                    yew::platform::spawn_local(async move {
                        maplibre::async_yield().await;
                        let recs = apply_filters(&filters, &records);
                        maplibre::update_data(
                            (*map).clone().unwrap(),
                            &recs,
                            *map_style.marker_size,
                            *map_style.line_size,
                            &map_style.colored_datastream,
                        )
                        .await;
                    })
                }
            },
            (records.clone(), filters.clone(), map_initialized.clone()),
        )
    };

    // === Restyle map === //

    // We do a full map restyling since any change to the base layer will cause
    // our source and layer to be removed. If only updating other map style
    // components, update_data and restyle_layer are sufficient. But a full
    // restyle is not too costly and handles all cases, so this is what we do.
    {
        // clippy warnings suppressed by not cloning values that can be moved in
        let map = map.clone();
        // let map_initialized = map_initialized.clone();
        let records = records.clone();
        // let basemap = basemap.clone();
        let filters = filters.clone();
        use_effect_with_deps(
            move |map_style| {
                if *map_initialized {
                    let map = map.clone();
                    let map_style = map_style.clone();
                    let basemap = basemap.clone();
                    let records = records.clone();
                    let filters = filters.clone();
                    yew::platform::spawn_local(async move {
                        maplibre::async_yield().await;
                        let recs = apply_filters(&filters, &records);

                        // full map restyle to change the base layer
                        map_initialized.set(false);
                        let on_style = {
                            let map_initialized = map_initialized.clone();
                            Box::new(move || map_initialized.set(true))
                        };
                        maplibre::restyle(
                            (*map).clone().unwrap(),
                            &recs,
                            &basemap,
                            *map_style.marker_size,
                            *map_style.line_size,
                            &map_style.solid_color,
                            &map_style.colored_datastream,
                            on_style,
                        )
                        .await;
                    });
                }
            },
            map_style.clone(),
        )
    };

    // === Re-center Plot on Click === //

    let flytodata_onclick = {
        let map = map.clone();
        let records = records.clone();
        let filters = filters.clone();
        Callback::from(move |_e: MouseEvent| {
            log::debug!("fly to data clicked");
            let recs = apply_filters(&filters, &records);
            maplibre::fly_to_data((*map).clone().unwrap(), &recs)
        })
    };

    let last_loc = use_selector(|state: &UIState| state.last_location.clone());
    let flytome_onclick = {
        // let map = map.clone();
        Callback::from(move |_e: MouseEvent| {
            log::debug!("fly to me clicked");
            if let Some(loc) = &*last_loc {
                maplibre::fly_to(
                    (*map).clone().unwrap(),
                    (loc.lon, loc.lat),
                    16.,
                );
            }
        })
    };

    html! {
        <>
        <div id={plot_id} class="w-screen flex-1 min-h-0 relative z-0">
            <div onclick={flytodata_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[3.375rem] left-2.5 z-40">
                <Icon icon_id={IconId::BootstrapBoundingBoxCircles}
                    class="h-6 w-6 text-[#aaaaaa]" />
            </div>
            <div onclick={flytome_onclick}
                class="p-2 rounded-lg bg-black w-min opacity-50 \
                    absolute bottom-[6.125rem] left-2.5 z-40">
                <Icon icon_id={IconId::FontAwesomeSolidLocationArrow}
                    class="h-6 w-6 text-[#aaaaaa]" />
            </div>
        </div>
        if map_style.colored_datastream.is_some()
            && *map_style.colored_datastream != ColoredDataStream::Time {
            <Colorbar records={records}
                filters={filters.clone()}
                colored_datastream={map_style.colored_datastream.clone()}/>
        }
        </>
    }
}

/// Return the (lng, lat) and text to show in the popup, given the current
/// record list and the lng and lat coordinates of the click.
fn get_hovertext(
    filtered_records: Rc<RefCell<Vec<common::Location>>>,
    lng: f64,
    lat: f64,
) -> ((f64, f64), String) {
    let local_offset = time::UtcOffset::current_local_offset().unwrap();
    let distances: Vec<_> = filtered_records
        .borrow()
        .iter()
        .map(|loc| (loc.lat - lat).abs() + (loc.lon - lng).abs())
        .collect();
    let mut argmin = 0;
    // assume the records being plotted have at least one element
    let mut min_distance = distances[0];
    for (i, d) in distances.iter().enumerate() {
        if *d < min_distance {
            min_distance = *d;
            argmin = i;
        }
    }
    let loc = &filtered_records.borrow()[argmin];
    (
        (loc.lon, loc.lat),
        format!(
            "{:.6}°, {:.6}°\
        <br>+/-{:.2} m, {:.2} m/s, {:.2}°\
        <br>{}",
            loc.lat,
            loc.lon,
            loc.accuracy,
            loc.speed,
            loc.course,
            loc.datetime
                .to_offset(local_offset)
                .format(&time::format_description::well_known::Rfc2822)
                .unwrap()
        ),
    )
}
