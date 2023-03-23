//! Analysis of collected data.

use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::common::{Location, TimeRange};
use crate::components::{
    NavbarWrapper, DATETIME_INPUT_STYLE, PRIMARY_BUTTON_STYLE,
    SECONDARY_BUTTON_STYLE,
};
use crate::viz;
use crate::websocket::{ToBack, ToFront, WebsocketService};

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
    let wss = use_context::<WebsocketService>().unwrap();

    let records = use_state(Vec::<Location>::new);
    let on_backend_msg = {
        let records = records.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationTimeRange(_time_range, locations) = msg {
                log::debug!("num records: {}", locations.len());
                records.set(locations.clone());
            }
        }
    };
    let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    wss.subscribe(*id, Box::new(on_backend_msg));

    // format used to put a time::OffsetDatetime into an HtmlInputElement
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]")
            .unwrap();
    let local_offset = time::UtcOffset::current_local_offset().unwrap();

    let time_range = use_state(time_range_today);
    let start_input = use_node_ref();
    let end_input = use_node_ref();

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

    let showmap_onclick = {
        let start_input = start_input.clone();
        let end_input = end_input.clone();
        let format = format.clone();
        Callback::from(move |_e: MouseEvent| {
            let start_input_value =
                start_input.cast::<HtmlInputElement>().unwrap().value();
            let end_input_value =
                end_input.cast::<HtmlInputElement>().unwrap().value();
            let start =
                time::PrimitiveDateTime::parse(&start_input_value, &format)
                    .unwrap()
                    .assume_offset(local_offset);
            let end = time::PrimitiveDateTime::parse(&end_input_value, &format)
                .unwrap()
                .assume_offset(local_offset);
            wss.send_msg(ToBack::GetLocationTimeRange(TimeRange {
                start,
                end,
            }));
        })
    };

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
                    <input type="datetime-local" ref={start_input} id="start"
                        value={time_range.start.format(&format).unwrap()}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)} />
                </div>
                <div class="flex items-center justify-between">
                    <label for="end">{"End Time"}</label>
                    <input type="datetime-local" ref={end_input} id="end"
                        value={time_range.end.format(&format).unwrap()}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)} />
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

            <br />
            <button onclick={showmap_onclick}
                class={format!("m-1 {}", PRIMARY_BUTTON_STYLE)}>
                    {"Update Map"}
            </button>

            <br />
            <PlotComponent records={(*records).clone()}/>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct Props {
    records: Vec<Location>,
}

#[function_component]
fn PlotComponent(Props { records }: &Props) -> Html {
    /*
    let first = Location {
        lat: 37.59,
        lon: -122.09,
        accuracy: 0.,
        speed: 0.,
        course: 0.,
        datetime: time::OffsetDateTime::now_local().unwrap(),
    };
    let second = Location {
        lat: 37.6,
        lon: -122.09,
        ..first
    };
    let third = Location {
        lat: 37.6,
        lon: -122.1,
        ..first
    };
    let fourth = Location {
        lat: 37.59,
        lon: -122.1,
        ..first
    };
    let records = vec![first, second, third, fourth];
    */
    let id = "plot-div";
    // TODO: reduce cloning?
    // TODO: choose second at the end of the minute by default?
    let plot = viz::gen_viz(records.clone(), 1., 0.27, 0.0, 0.8);
    yew::platform::spawn_local(async move {
        plotly::bindings::new_plot(id, &plot).await;
    });
    html! {
        <div id="plot-div" class="w-screen max-h-100 mt-4"></div>
    }
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
