//! Analysis of collected data.

use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::common::{Location, TimeRange};
use crate::components::{
    NavbarWrapper, DATETIME_INPUT_STYLE, SECONDARY_BUTTON_STYLE,
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

            <ShowMap />
        </NavbarWrapper>
    }
}

#[function_component]
fn ShowMap() -> Html {
    let records = use_state(Vec::<Location>::new);
    let on_backend_msg = {
        let records = records.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                records.set(locations.clone());
            }
        }
    };
    use_backend_event(on_backend_msg);

    // format used to put a time::OffsetDatetime into an HtmlInputElement
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]")
            .unwrap();

    let time_range = use_state(time_range_today);

    let start_onchange = {
        let time_range = time_range.clone();
        Callback::from(move |e: Event| {
            let start_elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val = parse_datetime_input(&start_elem.value());
            time_range.set(TimeRange {
                start: val,
                ..*time_range
            });
        })
    };
    let end_onchange = {
        let time_range = time_range.clone();
        Callback::from(move |e: Event| {
            let end_elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val = parse_datetime_input(&end_elem.value())
                + time::Duration::seconds(59); // get all data in the min
            time_range.set(TimeRange {
                end: val,
                ..*time_range
            });
        })
    };

    let week_onclick = {
        let time_range = time_range.clone();
        Callback::from(move |_e: MouseEvent| time_range.set(time_range_week()))
    };
    let day_onclick = {
        let time_range = time_range.clone();
        Callback::from(move |_e: MouseEvent| time_range.set(time_range_day()))
    };
    let today_onclick = {
        let time_range = time_range.clone();
        Callback::from(move |_e: MouseEvent| time_range.set(time_range_today()))
    };

    let wss = use_context::<WebsocketService>().unwrap();
    // Ask for new location data from backend after render anytime time_range
    // changes
    use_effect_with_deps(
        move |time_range| {
            wss.send_msg(ToBack::GetLocationTimeRange((**time_range).clone()));
        },
        time_range.clone(),
    );

    html! {
        <>
            // containing div for both datetime pickers to align to
            <div class="flex justify-center">
            <div class="max-w-fit">
                // items-center: align items to be centered vertically
                // justify-between: push elements away from each other so they
                //      align with the edges of the div containing both flexes
                <div class="flex items-center justify-between">
                    <label for="start">{"Start Time"}</label>
                    <input type="datetime-local" id="start"
                        value={time_range.start.format(&format).unwrap()}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)}
                        onchange={start_onchange} />
                </div>
                <div class="flex items-center justify-between">
                    <label for="end">{"End Time"}</label>
                    <input type="datetime-local" id="end"
                        value={time_range.end.format(&format).unwrap()}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)}
                        onchange={end_onchange}/>
                </div>
            </div>
            </div>

            <button onclick={week_onclick}
                class={format!("m-1 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Past 7 Days"}
            </button>
            <button onclick={day_onclick}
                class={format!("m-1 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Past 24 Hours"}
            </button>
            <button onclick={today_onclick}
                class={format!("m-1 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Today"}
            </button>

            <PlotComponent records={records} time_range={time_range}/>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct Props {
    records: UseStateHandle<Vec<Location>>,
    time_range: UseStateHandle<TimeRange>,
}

#[function_component]
fn PlotComponent(
    Props {
        records,
        time_range,
    }: &Props,
) -> Html {
    let plot_id = "plot-div";
    use_memo(
        |records| {
            let marker = viz::Marker::new()
                .opacity(0.8)
                .color(viz::color::Rgba::new(255, 64, 0, 0.8));
            let plot = viz::gen_viz(records.to_vec(), marker.clone());
            yew::platform::spawn_local(async move {
                plotly::bindings::new_plot(plot_id, &plot).await;
            });
        },
        records.clone(), // show a new plot only when records change
    );

    // Live updates to the plot free panning/zooming events, so we detect when
    // those events are occuring and disable live updates until after.
    let is_panning = use_state(|| false);
    let onpointerdown = {
        let is_panning = is_panning.clone();
        Callback::from(move |_| is_panning.set(true))
    };
    let onpointerup = {
        let is_panning = is_panning.clone();
        Callback::from(move |_| is_panning.set(false))
    };

    // Backlog of data points to show if panning/zooming was active
    let backlog = use_state(Vec::new);
    let on_backend_msg = {
        let time_range = time_range.clone();
        let is_panning = is_panning.clone();
        let backlog = backlog.clone();
        move |msg: &ToFront| {
            if let ToFront::LastLocation(location) = msg {
                if time_range.contains(&location.datetime) {
                    let mut new_backlog = (*backlog).clone();
                    new_backlog.push(location.clone());
                    if *is_panning {
                        // panning/zooming active, just update backlog
                        backlog.set(new_backlog);
                    } else {
                        // display new data and clear backlog
                        plotly_wasm::extend_traces_scattermapbox(
                            plot_id,
                            new_backlog.iter().map(|x| x.lat).collect(),
                            new_backlog.iter().map(|x| x.lon).collect(),
                        );
                        backlog.set(Vec::new());
                    }
                }
            }
        }
    };
    use_backend_event_with_deps(
        on_backend_msg,
        (time_range.clone(), is_panning.clone(), backlog.clone()),
    );

    html! {
        <div id={plot_id} class="w-screen max-h-96 mt-4"
            onpointerdown={onpointerdown} onpointerup={onpointerup}>
        </div>
    }
}

/// Parse the datetime received from a type="datetime-local" html input.
fn parse_datetime_input(val: &str) -> time::OffsetDateTime {
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]")
            .unwrap();
    let local_offset = time::UtcOffset::current_local_offset().unwrap();
    time::PrimitiveDateTime::parse(val, &format)
        .unwrap()
        .assume_offset(local_offset)
}

/// Get a time range for today up until now
fn time_range_today() -> TimeRange {
    let now = time::OffsetDateTime::now_local().unwrap();
    let start = now.replace_time(time::Time::MIDNIGHT);
    let end = now.replace_time(time::Time::from_hms(23, 59, 59).unwrap());
    TimeRange { start, end }
}

/// Get a time range for today up until now
fn time_range_day() -> TimeRange {
    let end = time::OffsetDateTime::now_local().unwrap();
    let start = end - time::Duration::DAY;
    TimeRange { start, end }
}

/// Get a time range for the past week
fn time_range_week() -> TimeRange {
    let now = time::OffsetDateTime::now_local().unwrap();
    let start = now - time::Duration::WEEK;
    let end = now;
    TimeRange { start, end }
}
