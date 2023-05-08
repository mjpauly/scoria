//! Analysis of collected data.

use plotly::common::Marker;
use wasm_bindgen::prelude::*;
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
use crate::plots::plotly_binds::MapboxRelayoutData;
use crate::plots::{
    cmaps, maps, plotly_binds, scatter_mapbox_update::ScatterMapboxUpdate,
};
use crate::ui_state::UIState;
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};
use common::{Location, TimeRange};

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
            r: 10,
            g: 132,
            b: 255,
            a: 1.0,
        }),
        marker_size: use_state(|| 6_usize),
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
    let relayout_data = use_state(|| Option::<MapboxRelayoutData>::None);
    // Collect the set of active filters, which is used to determine if the plot
    // is reloaded. This way editing an inactive filter doesn't change the plot.
    let active_filters: Vec<_> = (*filters)
        .clone()
        .into_iter()
        .filter(|filt| filt.enabled)
        .collect();

    // Ask for new location data from backend after render anytime time_range,
    // map_style, or active_filters changes
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        move |(time_range, ..)| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        // things that will cause a full plot reload if changed:
        (time_range.clone(), map_style.clone(), active_filters),
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
    relayout_data: UseStateHandle<Option<MapboxRelayoutData>>,
}

#[function_component]
fn PlotComponent(
    PlotComponentProps {
        time_range,
        map_style,
        filters,
        relayout_data,
    }: &PlotComponentProps,
) -> Html {
    let use_epsln_tile_server =
        use_selector(|s: &UIState| s.use_epsln_tile_server);
    // Show new plot if time range changes and new data comes from backend

    let plot_id = "map-div";
    let plot_initialized = use_state(|| false);
    let on_backend_msg = {
        let plot_initialized = plot_initialized.clone(); // only exports values
        let map_style = map_style.clone();
        let filters = filters.clone();
        let relayout_data = relayout_data.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                let locations = apply_filters(&filters, locations);
                crate::plots::maplibre::map_plot(
                    plot_id,
                    &locations,
                    map_style.basemap_style.get_url(*use_epsln_tile_server),
                );
                /*
                let marker = get_plot_marker(&locations, map_style.clone());
                plot_initialized.set(false);
                plotly_binds::new_plot(
                    plot_id,
                    &maps::map_plot(
                        locations,
                        marker,
                        map_style
                            .basemap_style
                            .to_plotly(*use_epsln_tile_server),
                        (*relayout_data).clone(),
                    ),
                );
                plot_initialized.set(true);

                // add pan/zoom event listener, so we can go back to the
                // previous zoom/pan/tilt/rotate view when making a new plot
                let relayout_data = relayout_data.clone();
                let cb = Box::new(move |v: JsValue| {
                    let result =
                        serde_wasm_bindgen::from_value::<MapboxRelayoutData>(v);
                    if let Ok(data) = result {
                        relayout_data.set(Some(data));
                    }
                });
                plotly_binds::add_plot_event_listener(
                    plot_id,
                    "plotly_relayout",
                    cb,
                );
                */
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (map_style.clone(), filters.clone()),
    );

    // Update plot with new data points without creating a new plot

    // Live updates to the plot freezes panning/zooming events, so we detect
    // when those events are occuring and disable live updates until after.
    let is_panning = use_state(|| false);
    let onpointerdown = {
        let is_panning = is_panning.clone();
        Callback::from(move |_| is_panning.set(true))
    };
    let onpointerup = {
        let is_panning = is_panning.clone();
        Callback::from(move |_| is_panning.set(false))
    };

    // Backlog of data points to show after panning/zooming finishes
    let backlog = use_state(Vec::new);
    let on_backend_msg = {
        let time_range = time_range.clone();
        let is_panning = is_panning.clone();
        let backlog = backlog.clone();
        let plot_initialized = plot_initialized.clone();
        move |msg: &ToFront| {
            // if new location data streamed in
            if let ToFront::LastLocation(location) = msg {
                // if time range of data we're displaying contains the new data
                if time_range.contains(&location.datetime) {
                    let mut new_backlog = (*backlog).clone();
                    new_backlog.push(location.clone());
                    if *is_panning || !*plot_initialized {
                        // panning/zooming active, or the plot does not yet
                        // exist -> just update backlog
                        backlog.set(new_backlog);
                    } else {
                        // not panning; display new data and clear backlog
                        let update = ScatterMapboxUpdate::new(
                            new_backlog.iter().map(|x| x.lat).collect(),
                            new_backlog.iter().map(|x| x.lon).collect(),
                            // TODO: use solid marker color for new data
                            // play with jsfiddle first
                            // maps::Rgba::new(255, 64, 0, 1.0),
                        );
                        plotly_binds::extend_trace(plot_id, update);
                        backlog.set(Vec::new());
                    }
                }
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (time_range.clone(), is_panning, backlog, plot_initialized),
    );

    html! {
        <div id={plot_id} class="w-screen flex-1 min-h-0"
            onpointerdown={onpointerdown} onpointerup={onpointerup}>
        </div>
    }
}

/// Get the marker for a plot given the vector of locations and the desired map
/// style.
fn get_plot_marker(locations: &[&Location], map_style: MapStyle) -> Marker {
    if *map_style.colored_datastream == ColoredDataStream::None {
        // No coloring based on data, just use solid color
        let marker = Marker::new()
            .color(map_style.solid_color.as_plotly())
            .size(*map_style.marker_size);
        return marker;
    }
    // Coloring based on data, using a colormap
    let colorvec = maps::Colorvec(
        locations
            .iter()
            .map(|x| map_style.colored_datastream.get_stream(x))
            .collect::<Vec<_>>(),
    );
    let mut marker = Marker::new()
        .color(colorvec)
        .auto_color_scale(false) // don't use plotly's default color scale
        .show_scale(true) // show the colorbar
        .opacity(map_style.solid_color.a)
        .size(*map_style.marker_size);
    let colorbar = maps::map_colorbar().title(
        plotly::common::Title::new(&map_style.colored_datastream.to_string())
            .side(plotly::common::Side::Top),
    );
    if *map_style.colored_datastream == ColoredDataStream::Course {
        // special case for circular cmap
        marker = marker
            .color_scale(cmaps::twilight_plotly())
            .cmin(0.0)
            .cmax(360.0)
            .color_bar(colorbar.dtick(90.0));
    } else {
        marker = marker
            .color_scale(cmaps::plasma_plotly())
            .color_bar(colorbar);
    }
    marker
}
