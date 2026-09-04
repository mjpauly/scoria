//! The dashboard page that shows metadata metrics, such as total distance
//! covered, min/max speeds, and starting and ending times.

use common::map_style::ColoredDataStream;
use common::mounted::MountID;
use common::state::MapSettingsTab;
use common::time_range::TimeDeltaRange;
use common::timeline::PeriodKind;
use common::units::time::format_date;
use common::LngLat;
use jiff::{civil::Date, SignedDuration, Zoned};
use serde_json::{json, Value};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::time_range_picker::update_time_range;
use crate::components::{BouncySavedScrollContainer, SECONDARY_BUTTON_STYLE};
use crate::components::{TabBar, TopNav};
use crate::maplibre::val_to_jsval;
use crate::plotly;
use crate::router::Route;
use crate::ui_state::{BackState, DerivedState, FrontState};
use crate::unwrapping::unwrap_result_or_log;

#[function_component]
pub fn MetricsDashboard() -> Html {
    html! {
        <>
            <TopNav>
                <></>
            </TopNav>
            <BouncySavedScrollContainer
                class="text-left max-w-prose"
                id="metrics-page"
            >
                <DefaultMetrics />
                // <ColoredTimeSeriesPlot />
                <Timeline />
            </BouncySavedScrollContainer>
            <TabBar />
        </>
    }
}

#[function_component]
fn DefaultMetrics() -> Html {
    let metrics = use_selector(|s: &DerivedState| s.dashboard_metrics.clone());

    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);

    // let f_count = format!("{} points", metrics.count);
    let unwrap_or_na =
        |maybe_x: Option<String>| maybe_x.unwrap_or_else(|| "N/A".into());
    let f_distance =
        unit_pref.format_length(metrics.total_distance, Some(0), Some(3));
    let f_dwell_time =
        humantime::format_duration(metrics.dwell_time.unsigned_abs())
            .to_string();
    let f_movement_time =
        humantime::format_duration(metrics.movement_time.unsigned_abs())
            .to_string();
    let format_speed = |speed: Option<f64>| {
        unwrap_or_na(speed.map(|s| unit_pref.format_velocity(s, Some(2))))
    };
    let f_min_speed = format_speed(metrics.min_speed);
    let f_max_speed = format_speed(metrics.max_speed);
    let f_avg_speed = format_speed(metrics.avg_speed);
    html! {
        <div class="px-4">
            <h1 class="text-primary text-3xl my-6 text-center">
                {"Stats"}
            </h1>

            <p class="text-neutral-500 mb-4">
                {"Computed for visible map data."}
            </p>

            <div class="grid grid-cols-3 gap-4 mb-4">
                <ScalarMetric title="Distance" value={f_distance} />
                <ScalarMetric title="Dwell Time" value={f_dwell_time} />
                <ScalarMetric title="Movement Time" value={f_movement_time} />
                <ScalarMetric title="Min Speed" value={f_min_speed} />
                <ScalarMetric title="Max Speed" value={f_max_speed} />
                <ScalarMetric title="Average Speed" value={f_avg_speed} />
                // <ScalarMetric title="Count" value={f_count} />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ScalarMetricProps {
    #[prop_or_default]
    pub class: Classes,
    pub title: AttrValue,
    pub value: AttrValue,
}

#[function_component]
fn ScalarMetric(p: &ScalarMetricProps) -> Html {
    html! {
        <div
            class={classes!(
                Classes::from("h-24 text-center flex flex-col bg-neutral-900 \
                              rounded-lg p-2"),
                p.class.clone()
            )}
        >
            <div class="my-auto">
                <p class="text-neutral-400 text-sm"> { p.title.clone() } </p>
                <p class=""> { p.value.clone() } </p>
            </div>
        </div>
    }
}

#[function_component]
fn Timeline() -> Html {
    let timeline = use_selector(|s: &DerivedState| s.timeline.clone());
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let time_pref = use_selector(|s: &FrontState| s.time_pref.clone());

    let format_dur = |dur: SignedDuration| {
        humantime::format_duration(dur.unsigned_abs()).to_string()
    };

    let navigator = use_navigator().unwrap();
    let front_dispatch = Dispatch::<FrontState>::new();
    let create_pin = {
        let navigator = navigator.clone();
        front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, lnglat: LngLat| {
                state.map.selected_pin_id = None;
                state.map.current_pin = Default::default();
                state.map.current_pin.lnglat = lnglat;
                state.map.settings_tab = MapSettingsTab::PinDetails;
                state.map.editable_pin = true;
                // center view on the new pin
                state.map.view_pos.center = lnglat;
                state.map.view_pos.zoom = 16.0;
                navigator.push(&Route::Analyze)
            },
        )
    };

    let show_details = use_state(|| Option::<usize>::None);

    let map_tz = use_selector(|s: &BackState| s.map_tz.clone());
    let new_day_html = |date: Date| {
        let day = format_date(&date).unwrap_or_else(|_| "New Day".to_string());
        let map_tz = map_tz.clone();
        let day_onclick =
            front_dispatch.reduce_mut_callback(move |s: &mut FrontState| {
                s.map.time_delta_range =
                    unwrap_result_or_log!(TimeDeltaRange::date(date, &map_tz));
                update_time_range(s, &map_tz);
            });
        html! {
            <button
                class="text-left text-neutral-500 mt-2 mb-1"
                onclick={day_onclick}
            >
                {"🗓️ "}{day}
            </button>
        }
    };

    // `scan` lets us carry the previous date forward as state
    let elems = timeline.iter().enumerate().scan(
        (None, None),
        // prev_date is the date when the date was last displayed
        // prev_time_formatted is the last formatted time, used to determine if
        // the UTC offset has changed and should thus be displayed.
        move |(prev_date, prev_time_formatted), (i, period)| {
            let start = &period.start;
            let end = &period.end;
            // whether to display the new day
            let new_day = prev_date
                .as_ref()
                .map(|p: &Zoned| (p.date() != start.date()))
                .unwrap_or(true)
                .then(|| start.date())
                .map(new_day_html);
            if new_day.is_some() {
                *prev_date = Some(start.clone());
            }
            let mut format_time = |t: &Zoned| {
                let date_ref = prev_date.as_ref().map(|zdt: &Zoned| zdt.date());
                let tz_ref = prev_time_formatted.as_ref().map(
                    |zdt: &Zoned| zdt.time_zone()
                );
                let out = time_pref
                    .format_jiff_time_with_previous(t, &tz_ref, &date_ref)
                    .unwrap_or_else(|_| "?".to_string());
                *prev_time_formatted = Some(t.clone());
                out
            };

            Some(match &period.kind
    {
        PeriodKind::Dwell(dwell) => {
            // display a new day marker after the element where the day changed
            let start_time = format_time(start);
            let end_time = format_time(end);
            let f_dur = format_dur(start.duration_until(end));
            let lnglat = dwell.lnglat;
            let create_pin_onclick = {
                let create_pin = create_pin.clone();
                Callback::from(move |_e: MouseEvent| create_pin.emit(lnglat))
            };
            let show_details_onclick = {
                let show_details = show_details.clone();
                Callback::from(move |_e: MouseEvent| {
                    if *show_details != Some(i) {
                        show_details.set(Some(i));
                    } else {
                        show_details.set(None);
                    }
                })
            };
            let f_lnglat = html!{
                <>
                    // preserve whitespace but also wrap
                    <span class="inline-block whitespace-pre-wrap">
                        {format!(
                            "{}, ",
                            unit_pref.format_angle(lnglat.lat, Some(5))
                        )}
                    </span>
                    <span class="inline-block">
                        {unit_pref.format_angle(lnglat.lng, Some(5))}
                    </span>
                </>
            };
            let f_deviation = format!(
                "±{}",
                unit_pref.format_small_length(dwell.deviation, Some(2)),
            );
            html! {
                <>
                {new_day}
                <button
                    onclick={show_details_onclick}
                    class="flex flex-col gap-1 bg-neutral-900 rounded-lg \
                    py-2 px-4"
                >
                    // header
                    <div class="text-sm text-neutral-500 flex justify-between \
                        w-full gap-4 font-medium"
                    >
                        // grow, but have a short basis to preferentially wrap
                        <span class="text-left basis-1/2 grow">
                            { start_time }
                        </span>
                        <span class="text-right">{ f_dur }</span>
                    </div>
                    // main details
                    <div class="flex flex-col gap-1 w-full">
                        <div class="flex justify-between items-center w-full \
                            gap-2"
                        >
                            <span class="text-left">
                                if let Some((pin, _)) = &dwell.detected_pin {
                                    { format!("{} {}", pin.icon, pin.name) }
                                } else {
                                    { f_lnglat.clone() }
                                }
                            </span>
                            <Icon
                                icon_id={
                                    if *show_details == Some(i) {
                                        IconId::BootstrapChevronDown
                                    } else {
                                        IconId::BootstrapChevronLeft
                                    }
                                }
                                class="h-4 w-4 text-neutral-500"
                            />
                        </div>
                        if *show_details == Some(i) {
                            <div class="flex justify-between items-start \
                                w-full gap-4"
                            >
                                <div class="flex flex-col text-neutral-500 \
                                    text-left text-sm justify-start"
                                >
                                    if let Some((_pin, dist)) =
                                        &dwell.detected_pin
                                    {
                                        <span>
                                            <span
                                                class="inline-block"
                                            >
                                                {f_lnglat}
                                            </span>
                                            {" "}
                                            <span
                                                class="inline-block"
                                            >
                                                {f_deviation}
                                            </span>
                                        </span>
                                        <span>
                                            <span class="whitespace-pre-wrap">
                                                {"Distance to place: "}
                                            </span>
                                            <span class="inline-block">
                                                {unit_pref.format_small_length(
                                                    *dist,
                                                    Some(2)
                                                )}
                                            </span>
                                        </span>
                                    } else {
                                        <span>
                                            {format!( "{}", f_deviation)}
                                        </span>
                                    }
                                </div>
                                <div class="flex flex-col justify-end \
                                    items-end gap-2 flex-wrap"
                                >
                                    <button
                                        class={classes!(
                                            SECONDARY_BUTTON_STYLE.to_string(),
                                            "text-base px-2 py-1".to_string(),
                                        )}
                                        onclick={create_pin_onclick}
                                    >
                                        { "New Place" }
                                    </button>
                                </div>
                            </div>
                        }
                    </div>
                    // footer
                    <div class="text-sm text-neutral-500 flex justify-between \
                        w-full font-medium"
                    >
                        <span class="text-left">{ end_time }</span>
                        <span></span>
                    </div>
                </button>
                </>
            }
        }
        PeriodKind::Movement(movement) => {
            let f_dur = format_dur(start.duration_until(end));
            let f_distance =
                unit_pref.format_length(movement.distance, Some(0), Some(2));
            html! {
                <>
                    {new_day}
                    <div class="flex gap-4 px-2 text-neutral-500">
                        <span class="bg-neutral-500 w-1 rounded-full" />
                        <span>
                            { f_distance }
                        </span>
                        <span class="flex-grow" />
                        <span class="text-sm"> { f_dur } </span>
                    </div>
                </>
            }
        }
        PeriodKind::Unknown => {
            html! {
                <>
                    {new_day}
                    <hr class="border border-dashed border-neutral-500 mx-2" />
                </>
            }
        }
    })});

    html! {
        <div class="px-4">
            <h1 class="text-primary text-3xl my-6 text-center">
                {"Timeline"}
            </h1>

            <div class="flex flex-col gap-2 mb-4">
                { for elems }
            </div>
        </div>
    }
}

#[function_component]
pub fn ColoredTimeSeriesPlot() -> Html {
    let ids_raw_data =
        use_selector(|s: &DerivedState| s.colored_timeseries_plot.clone());
    let colored_datastream =
        use_selector(|s: &FrontState| s.map.style.colored_datastream);
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let mut ylabel = None;
    let data: Value = ids_raw_data
        .iter()
        .map(|(mount_id, raw_data)| {
            ylabel = Some(raw_data.ylabel.clone());
            let str_dates = raw_data
                .t
                .iter()
                .map(plotly::time_to_str)
                .collect::<Vec<_>>();
            let y = if *colored_datastream == ColoredDataStream::TimeOfDay {
                plotly::sec_to_timeofday(raw_data.y.iter())
            } else if *colored_datastream == ColoredDataStream::Time {
                json!(str_dates)
            } else {
                json!(raw_data
                    .y
                    .iter()
                    .map(|x| colored_datastream
                        .to_preferred_units(&unit_pref, *x))
                    .collect::<Vec<_>>())
            };
            json!({
                "x": str_dates,
                "y": y,
                "type": "scatter",
                "mode": "markers",
                "name": db_name(mount_id),
                // "mode": "lines+markers",
            })
        })
        .collect();
    let layout = json!({
        "yaxis": {
            "tickformat": plotly::tickformat(&colored_datastream),
        },
        "height": 300,
        "margin": {
            "b": 40,
            "l": 40,
            "r": 40,
            "t": 20,
        },
        "modebar": {
            "remove": [
                "toimage", "lasso", "select", "zoomin", "zoomout",
            ],
        },
        // legend ref: https://plotly.com/javascript/legend/
        "showlegend": (ids_raw_data.len() > 1),
        "legend": {
            "xanchor": "right",
            "x": 1,
            "y": 0,
            "bgcolor": "#111111aa",
        },
        "template": plotly::dark_template(),
    });
    let config = json!({
        "responsive": true,
        "displaylogo": false,
    });
    let plot_id = "timeseries-div";
    use_effect_with_deps(
        move |d| {
            plotly::react(
                plot_id,
                &val_to_jsval(d),
                &val_to_jsval(&layout),
                &val_to_jsval(&config),
            );
            // Purge on cleanup: responsive:true adds a window resize
            // listener that otherwise retains the div and the detached
            // page tree (see doc/decimation/probe-results/churn.md). The
            // element is captured now since it may be detached by cleanup.
            let gd = plotly::graph_div(plot_id);
            move || {
                if let Some(gd) = gd {
                    plotly::purge_element(&gd)
                }
            }
        },
        data,
    );
    html! {
        <div id={plot_id}>
        </div>
    }
}

fn db_name(id: &MountID) -> String {
    Dispatch::<FrontState>::new()
        .get()
        .mounted_db_settings
        .get(id)
        .map(|setting| setting.name.clone())
        .unwrap_or_else(|| id.to_string())
}
