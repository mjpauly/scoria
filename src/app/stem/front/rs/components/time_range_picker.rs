//! Picker element for a date and time range, including convenience buttons for
//! today, the past 24 hours, and the past 7 days.

use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::common::TimeRange;
use crate::components::{DATETIME_INPUT_STYLE, SECONDARY_BUTTON_STYLE};

#[derive(Properties, PartialEq)]
pub struct TimeRangePickerProps {
    pub time_range: UseStateHandle<TimeRange>,
}

#[function_component]
pub fn TimeRangePicker(
    TimeRangePickerProps { time_range }: &TimeRangePickerProps,
) -> Html {
    // format used to put a time::OffsetDatetime into an HtmlInputElement
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]")
            .unwrap();

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
                + time::Duration::seconds(59); // get all data in the minute
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
    html! {
        <div class="overflow-y-auto">
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
pub fn time_range_today() -> TimeRange {
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
