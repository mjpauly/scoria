//! Analysis of collected data.

use yew::prelude::*;

// use crate::common::{Location, TimeRange};
use crate::cmap;
use crate::common::TimeRange;
use crate::components::{
    map_styler::{BasemapStyle, ColoredDataStream, MapStyle, Rgba},
    time_range_picker::time_range_today,
    MapStyler, NavbarWrapper, TimeRangePicker, PRIMARY_BUTTON_STYLE,
    SECONDARY_BUTTON_STYLE,
};
use crate::plotly_wasm;
use crate::scatter_mapbox_update::ScatterMapboxUpdate;
use crate::viz;
use crate::websocket::{
    use_backend_event_with_deps, ToBack, ToFront, WebsocketService,
};

#[function_component]
pub fn Analyze() -> Html {
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
            r: 255,
            g: 64,
            b: 0,
            a: 0.8,
        }),
        marker_size: use_state(|| 6 as usize),
        basemap_style: use_state(|| BasemapStyle::StamenTerrain),
        colored_datastream: use_state(|| ColoredDataStream::None),
    };

    // Ask for new location data from backend after render anytime time_range
    // changes
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        move |(time_range, ..)| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        // things that will cause a full plot reload if changed:
        (time_range.clone(), map_style.clone()),
    );

    let show_time_picker = use_state(|| false);
    let show_plot_styler = use_state(|| false);

    // after rerender, trigger the plot's resize handler if needs an update
    use_effect_with_deps(
        move |_| {
            let event = web_sys::Event::new("resize").unwrap();
            web_sys::window().unwrap().dispatch_event(&event).unwrap();
        },
        (show_time_picker.clone(), show_plot_styler.clone()),
    );

    html! {
        <div class="flex flex-col h-full">
            <PlotComponent
                time_range={time_range.clone()}
                map_style={map_style.clone()} />
            if *show_plot_styler {
                <MapStyler map_style={map_style.clone()} />
                <hr class="border-t-1 border-neutral-700" />
            }
            if *show_time_picker {
                <TimeRangePicker time_range={time_range.clone()} />
                <hr class="border-t-1 border-neutral-700" />
            }
            <SettingsPicker
                show_time_picker={show_time_picker.clone()}
                show_plot_styler={show_plot_styler.clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SettingsPickerProps {
    show_time_picker: UseStateHandle<bool>,
    show_plot_styler: UseStateHandle<bool>,
}

#[function_component]
fn SettingsPicker(
    SettingsPickerProps {
        show_time_picker,
        show_plot_styler,
    }: &SettingsPickerProps,
) -> Html {
    let time_onclick = {
        let show_time_picker = show_time_picker.clone();
        Callback::from(move |_e: MouseEvent| {
            show_time_picker.set(!*show_time_picker);
        })
    };
    let style_onclick = {
        let show_plot_styler = show_plot_styler.clone();
        Callback::from(move |_e: MouseEvent| {
            show_plot_styler.set(!*show_plot_styler)
        })
    };
    let style_button_style = if **show_plot_styler {
        PRIMARY_BUTTON_STYLE
    } else {
        SECONDARY_BUTTON_STYLE
    };
    let time_button_style = if **show_time_picker {
        PRIMARY_BUTTON_STYLE
    } else {
        SECONDARY_BUTTON_STYLE
    };
    html! {
        <div class="flex my-1">
            <div class="mx-auto">
                <button onclick={style_onclick}
                    class={format!("m-1 {}", style_button_style)}>
                        {"Map Style"}
                </button>
                <button onclick={time_onclick}
                    class={format!("m-1 {}", time_button_style)}>
                        {"Time Range"}
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
}

#[function_component]
fn PlotComponent(
    PlotComponentProps {
        time_range,
        map_style,
    }: &PlotComponentProps,
) -> Html {
    // Show new plot if time range changes and new data comes from backend

    let plot_id = "plot-div";
    let plot_initialized = use_state(|| false);
    let on_backend_msg = {
        let plot_initialized = plot_initialized.clone(); // only exports values
        let map_style = map_style.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                let marker;
                if *map_style.colored_datastream == ColoredDataStream::None {
                    marker = viz::Marker::new()
                        .color(map_style.solid_color.to_plotly())
                        .size(*map_style.marker_size);
                } else {
                    marker = viz::Marker::new()
                        .color(viz::Colorvec(
                            locations
                                .iter()
                                .map(|x| {
                                    map_style.colored_datastream.get_stream(&x)
                                })
                                .collect::<Vec<_>>(),
                        ))
                        // don't use plotly's default color scale
                        .auto_color_scale(false)
                        .color_scale(cmap::viridis_plotly()) // TODO: configure
                        .show_scale(true) // TODO: configure
                        .opacity(map_style.solid_color.a)
                        .size(*map_style.marker_size);
                }

                plot_initialized.set(false);
                plotly_wasm::new_plot(
                    plot_id,
                    &viz::map_plot(
                        locations.clone(),
                        marker,
                        map_style.basemap_style.to_plotly(),
                    ),
                );
                plot_initialized.set(true);
            }
        }
    };
    use_backend_event_with_deps(on_backend_msg, map_style.clone());

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
                            // viz::Rgba::new(255, 64, 0, 1.0),
                        );
                        plotly_wasm::extend_trace(plot_id, update);
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
