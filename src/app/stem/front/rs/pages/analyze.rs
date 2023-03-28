//! Analysis of collected data.

use yew::prelude::*;

// use crate::common::{Location, TimeRange};
use crate::common::TimeRange;
use crate::components::{
    time_range_picker::time_range_today, NavbarWrapper, TimeRangePicker,
};
use crate::plotly_wasm;
use crate::viz;
use crate::websocket::{
    use_backend_event, use_backend_event_with_deps, ToBack, ToFront,
    WebsocketService,
};

#[function_component]
pub fn Analyze() -> Html {
    html! {
        <NavbarWrapper>
            <h1 class="text-sky-500 text-3xl mb-6">
                {"Analyze"}
            </h1>

            <AnalyzeLocation />
        </NavbarWrapper>
    }
}

#[function_component]
fn AnalyzeLocation() -> Html {
    let time_range = use_state(time_range_today);

    // Ask for new location data from backend after render anytime time_range
    // changes
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        move |time_range| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        time_range.clone(),
    );

    html! {
        <>
            <TimeRangePicker time_range={time_range.clone()} />

            <PlotComponent time_range={time_range} />
        </>
    }
}

#[derive(Properties, PartialEq)]
struct Props {
    time_range: UseStateHandle<TimeRange>,
}

#[function_component]
fn PlotComponent(Props { time_range }: &Props) -> Html {
    let plot_id = "plot-div";
    // Generate an empty plot with the right layout on first render
    use_effect_with_deps(
        |_| {
            let marker = viz::Marker::new()
                .color(viz::color::Rgba::new(255, 64, 0, 1.0));
            let plot = viz::empty_plot(marker);
            plotly_wasm::new_plot(plot_id, &plot);
        },
        (),
    );
    let on_backend_msg = {
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                // TODO: zoom and centering
                let update = plotly_wasm::ScatterMapboxUpdate::new(
                    locations.iter().map(|x| x.lat).collect(),
                    locations.iter().map(|x| x.lon).collect(),
                );
                plotly_wasm::restyle(plot_id, update);
                // log::debug!("update json: {}", update.to_json());
            }
        }
    };
    use_backend_event(on_backend_msg);

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
        move |msg: &ToFront| {
            // if new location data streamed in
            if let ToFront::LastLocation(location) = msg {
                // if time range of data we're displaying contains the new data
                if time_range.contains(&location.datetime) {
                    let mut new_backlog = (*backlog).clone();
                    new_backlog.push(location.clone());
                    if *is_panning {
                        // panning/zooming active, just update backlog
                        backlog.set(new_backlog);
                    } else {
                        // not panning; display new data and clear backlog
                        let update = plotly_wasm::ScatterMapboxUpdate::new(
                            new_backlog.iter().map(|x| x.lat).collect(),
                            new_backlog.iter().map(|x| x.lon).collect(),
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
        (time_range.clone(), is_panning, backlog),
    );

    html! {
        <div id={plot_id} class="w-screen max-h-96 mt-4"
            onpointerdown={onpointerdown} onpointerup={onpointerup}>
        </div>
    }
}
