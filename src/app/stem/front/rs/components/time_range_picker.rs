//! Picker element for a date and time range, including convenience buttons for
//! today, the past 24 hours, and the past 7 days.

use common::time_range::TimeDeltaRange;
use common::TimeRange;
use time::macros::{datetime, format_description};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{DATETIME_INPUT_STYLE, SECONDARY_BUTTON_STYLE};
use crate::ui_state::FrontState;

#[function_component]
pub fn TimeRangePicker() -> Html {
    // format used to put a time::OffsetDatetime into an HtmlInputElement
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]");

    let dispatch = Dispatch::<FrontState>::new();
    let time_range = use_selector(|s: &FrontState| s.map.time_range);

    let start_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let start_elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(val) = parse_datetime_input(&start_elem.value()) {
                s.map.time_range.start = val;
                // update the time_delta_range to match
                s.map.time_delta_range.start_offset =
                    val - time::OffsetDateTime::now_utc();
                s.map.time_delta_range.snap_start_to_day = false;
            }
        },
    );
    let end_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let end_elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(mut val) = parse_datetime_input(&end_elem.value()) {
                // get all data in the minute
                val += time::Duration::seconds(59);
                s.map.time_range.end = val;
                // update the time_delta_range to match
                s.map.time_delta_range.end_offset =
                    val - time::OffsetDateTime::now_utc();
                s.map.time_delta_range.snap_end_to_day = false;
            }
        },
    );

    let all_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_range = time_range_all();
            let now = time::OffsetDateTime::now_utc();
            s.map.time_delta_range = TimeDeltaRange {
                start_offset: s.map.time_range.start - now,
                end_offset: s.map.time_range.end - now,
                snap_start_to_day: true,
                snap_end_to_day: true,
                offset: Some(local_offset()),
            };
        });
    let week_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = time_delta_range_week();
            // update absolute range
            s.map.time_range = (&s.map.time_delta_range).into();
        });
    let day_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = time_delta_range_day();
            s.map.time_range = (&s.map.time_delta_range).into();
        });
    let today_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.map.time_delta_range = time_delta_range_today();
            s.map.time_range = (&s.map.time_delta_range).into();
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
                        value={time_range.start.format(&format).unwrap()}
                        class={format!("m-1 ml-4 {}", DATETIME_INPUT_STYLE)}
                        onchange={start_onchange} />
                </div>
                <div class="flex items-center justify-between flex-wrap">
                    <label for="end">{"End"}</label>
                    <input type="datetime-local" id="end"
                        value={time_range.end.format(&format).unwrap()}
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
        </div>
    }
}

pub fn local_offset() -> time::UtcOffset {
    time::UtcOffset::current_local_offset().unwrap()
}

/// Parse the datetime received from a type="datetime-local" html input.
fn parse_datetime_input(
    val: &str,
) -> Result<time::OffsetDateTime, time::error::Parse> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]");
    Ok(time::PrimitiveDateTime::parse(val, &format)?
        .assume_offset(local_offset()))
}

pub fn time_delta_range_today() -> TimeDeltaRange {
    TimeDeltaRange {
        start_offset: time::Duration::ZERO,
        end_offset: time::Duration::ZERO,
        snap_start_to_day: true,
        snap_end_to_day: true,
        offset: Some(local_offset()),
    }
}

fn time_delta_range_day() -> TimeDeltaRange {
    TimeDeltaRange {
        start_offset: -time::Duration::DAY,
        end_offset: time::Duration::ZERO,
        snap_start_to_day: false,
        snap_end_to_day: false,
        offset: Some(local_offset()),
    }
}

fn time_delta_range_week() -> TimeDeltaRange {
    TimeDeltaRange {
        start_offset: -time::Duration::WEEK,
        end_offset: time::Duration::ZERO,
        snap_start_to_day: false,
        snap_end_to_day: false,
        offset: Some(local_offset()),
    }
}

fn time_range_all() -> TimeRange {
    TimeRange {
        start: datetime!(2023-01-01 0:00).assume_offset(local_offset()),
        end: datetime!(2033-01-01 0:00).assume_offset(local_offset()),
    }
}
