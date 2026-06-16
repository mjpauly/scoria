//! Picker element for a date and time range, including convenience buttons for
//! today, the past 24 hours, and the past 7 days.

use anyhow::Context;
use common::time_range::TimeDeltaRange;
use jiff::{civil::DateTime, Timestamp, Zoned};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{DATETIME_INPUT_STYLE, SECONDARY_BUTTON_STYLE};
use crate::ui_state::{BackState, FrontState};
use crate::unwrapping::unwrap_result_or_log;
use crate::websocket::WebsocketService;

/// Format a timestamp in a timezone, falling back to UTC on failure
fn format_timestamp(ts: &Timestamp, tz: &str) -> String {
    let zdt = ts
        .intz(tz)
        .with_context(|| format!("converting to timezone {tz}"))
        .unwrap_or_else(|e| {
            tracing::error!("{e:?}");
            ts.intz("UTC").unwrap()
        });
    // format iso8601 up to second precision
    format!("{:.0}", zdt.datetime())
}

/// Parse the datetime received from a type="datetime-local" html input.
fn parse_datetime_input(val: &str, tz: &str) -> Result<Zoned, jiff::Error> {
    let dt: DateTime = val.parse()?;
    dt.intz(tz)
}

fn now(tz: &str) -> Zoned {
    Timestamp::now()
        .intz(tz)
        .unwrap_or_else(|_| Timestamp::now().intz("UTC").unwrap())
}

#[function_component]
pub fn TimeRangePicker() -> Html {
    let map_tz = use_selector(|s: &BackState| s.map_tz.clone());

    let dispatch = Dispatch::<FrontState>::new();
    let time_range = use_selector(|s: &FrontState| s.map.time_range);

    let start_time_formatted = format_timestamp(&time_range.start, &map_tz);
    let end_time_formatted = format_timestamp(&time_range.end, &map_tz);

    let start_onchange = {
        let map_tz = map_tz.clone();
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let start_elem: HtmlInputElement = e.target_dyn_into().unwrap();
                if let Ok(val) =
                    parse_datetime_input(&start_elem.value(), &map_tz)
                {
                    s.map.time_range.start = val.timestamp();
                    // update the time_delta_range to match
                    if let Ok(new_start_offset) = now(&map_tz).until(&val) {
                        s.map.time_delta_range.start_offset = new_start_offset;
                    }
                    s.map.time_delta_range.snap_start_to_day = false;
                }
            },
        )
    };
    let end_onchange = {
        let map_tz = map_tz.clone();
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let end_elem: HtmlInputElement = e.target_dyn_into().unwrap();
                if let Ok(val) =
                    parse_datetime_input(&end_elem.value(), &map_tz)
                {
                    s.map.time_range.end = val.timestamp();
                    // update the time_delta_range to match
                    if let Ok(new_end_offset) = now(&map_tz).until(&val) {
                        s.map.time_delta_range.end_offset = new_end_offset;
                    }
                    s.map.time_delta_range.snap_end_to_day = false;
                }
            },
        )
    };

    let (tz0, tz1, tz2, tz3) = (
        map_tz.clone(),
        map_tz.clone(),
        map_tz.clone(),
        map_tz.clone(),
    );
    let wss = use_context::<WebsocketService>().unwrap();
    let all_onclick =
        dispatch.reduce_mut_future_callback(move |s: &mut FrontState| {
            let wss = wss.clone();
            let tz0 = tz0.clone();
            Box::pin(async move {
                if let Ok(Some(tr)) = wss.get_db_full_time_range().await {
                    s.map.time_delta_range =
                        unwrap_result_or_log!(TimeDeltaRange::range(tr));
                    update_time_range(s, &tz0);
                }
            })
        });
    let week_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = TimeDeltaRange::week();
            update_time_range(s, &tz1);
        });
    let day_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = TimeDeltaRange::past_24h();
            update_time_range(s, &tz2);
        });
    let today_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = TimeDeltaRange::today();
            update_time_range(s, &tz3);
        });
    html! {
        <div class="my-1 overflow-scroll w-screen">
            // containing div for both datetime pickers to align to
            <div class="flex">
            <div class="max-w-fit mx-auto">
                // items-center: align items to be centered vertically
                // justify-between: push elements away from each other so they
                //      align with the edges of the div containing both flexes
                <div class="flex items-center justify-between flex-wrap">
                    <label for="start">{"Start"}</label>
                    <input type="datetime-local" id="start"
                        value={start_time_formatted}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)}
                        onchange={start_onchange} />
                </div>
                <div class="flex items-center justify-between flex-wrap">
                    <label for="end">{"End"}</label>
                    <input type="datetime-local" id="end"
                        value={end_time_formatted}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)}
                        onchange={end_onchange}/>
                </div>
            </div>
            </div>

            <button onclick={all_onclick} id="time_range_all_btn"
                class={format!("m-1 py-1.5 px-3 {}", SECONDARY_BUTTON_STYLE)}>
                    {"All"}
            </button>
            <button onclick={week_onclick} id="time_range_week_btn"
                class={format!("m-1 py-1.5 px-3 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Past 7d"}
            </button>
            <button onclick={day_onclick} id="time_range_day_btn"
                class={format!("m-1 py-1.5 px-3 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Past 24h"}
            </button>
            <button onclick={today_onclick} id="time_range_today_btn"
                class={format!("m-1 py-1.5 px-3 {}", SECONDARY_BUTTON_STYLE)}>
                    {"Today"}
            </button>

            // <p class="text-neutral-500 mx-2">
                // {"Time Zone: "}{&map_tz.0}
            // </p>
        </div>
    }
}

/// Update the map.time_range based on a new value for map.time_delta_range.
pub fn update_time_range(s: &mut FrontState, tz: &str) {
    if let Ok(new_range) = s
        .map
        .time_delta_range
        .to_time_range(tz, s.time_pref.day_separation_time)
    {
        s.map.time_range = new_range;
    }
}
