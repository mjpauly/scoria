//! The dashboard page that shows metadata metrics, such as total distance
//! covered, min/max speeds, and starting and ending times.

use common::map_style::ColoredDataStream;
use common::timeline::Period;
use common::units::time::format_datetime;
use serde_json::json;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::time_range_picker::local_offset;
use crate::components::BouncyScrollContainerBase;
use crate::components::{TabBar, TopNav};
use crate::maplibre::val_to_jsval;
use crate::plotly;
use crate::ui_state::{DerivedState, FrontState};

#[function_component]
pub fn MetricsDashboard() -> Html {
    html! {
        <>
            <TopNav>
                <></>
            </TopNav>
            <BouncyScrollContainerBase class="text-left">
                <DefaultMetrics />
                // <ColoredTimeSeriesPlot />
                <Timeline />
            </BouncyScrollContainerBase>
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
    let f_distance = unit_pref.format_length(metrics.total_distance, Some(3));
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

    let format_dur = |dur: time::Duration| {
        humantime::format_duration(dur.unsigned_abs()).to_string()
    };

    let elems = timeline.iter().scan(None, |prev_date, period| Some(match period {
        Period::Dwell(dwell) => {
            let offset = local_offset();
            let mut format_time = |t: time::OffsetDateTime| {
                let with_offset = t.to_offset(offset);
                let formatted = format_datetime(
                    &with_offset, prev_date, true)
                    .unwrap_or_else(|_| "?".to_string());
                *prev_date = Some(with_offset);
                formatted
            };
            let start_time = format_time(dwell.time.start);
            let end_time = format_time(dwell.time.end);
            let f_dur = format_dur(dwell.time.start - dwell.time.end);
            html! {
                <div class="flex flex-col gap-1 bg-neutral-900 rounded-lg \
                    py-2 px-4"
                >
                    <div class="text-sm text-neutral-500 flex justify-between">
                        <span>{ start_time }</span>
                        <span>{ f_dur }</span>
                    </div>
                    <p class="text-lg">
                        <span class="text-primary">
                            { unit_pref
                                .format_angle(dwell.lnglat.lat, Some(5))
                            }
                            {", "}
                            { unit_pref
                                .format_angle(dwell.lnglat.lng, Some(5))
                            }
                        </span>
                    </p>
                    <div class="text-sm text-neutral-500"> {end_time} </div>
                </div>
            }
        }
        Period::Movement(movement) => {
            let f_dur = format_dur(movement.time.start - movement.time.end);
            let f_distance =
                unit_pref.format_length(movement.distance, Some(3));
            html! {
                <div class="flex gap-4 px-2 text-neutral-500">
                    <span class="bg-neutral-500 w-1 rounded-full" />
                    <span>
                        { f_distance }
                    </span>
                    <span class="flex-grow" />
                    <span class="text-sm"> { f_dur } </span>
                </div>
            }
        }
        Period::Unknown => {
            html! {
                <hr class="border border-dashed border-neutral-500 mx-2" />
            }
        }
    }));

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
    let raw_data =
        use_selector(|s: &DerivedState| s.colored_timeseries_plot.clone());
    let colored_datastream =
        use_selector(|s: &FrontState| s.map.style.colored_datastream);
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let offset = local_offset();
    let str_dates = raw_data
        .t
        .iter()
        .map(|t| plotly::time_to_str(&t.to_offset(offset)))
        .collect::<Vec<_>>();
    let y = if *colored_datastream == ColoredDataStream::TimeOfDay {
        plotly::sec_to_timeofday(raw_data.y.iter())
    } else if *colored_datastream == ColoredDataStream::Time {
        json!(raw_data
            .y
            .iter()
            .map(|t| plotly::time_to_str(
                &time::OffsetDateTime::from_unix_timestamp(*t as i64)
                    .unwrap()
                    .to_offset(offset)
            ))
            .collect::<Vec<_>>())
    } else {
        json!(raw_data
            .y
            .iter()
            .map(|x| colored_datastream.to_preferred_units(&unit_pref, *x))
            .collect::<Vec<_>>())
    };
    let data = json!([{
        "x": str_dates,
        "y": y,
        "type": "scatter",
        "mode": "markers",
        // "mode": "lines+markers",
    }]);
    let layout = json!({
        "ylabel": {
            "text": raw_data.ylabel,
        },
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
            "remove": "toimage",
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
            // log::debug!("a");
            plotly::react(
                plot_id,
                &val_to_jsval(d),
                &val_to_jsval(&layout),
                &val_to_jsval(&config),
            );
            // log::debug!("bb");
        },
        data,
    );
    html! {
        <div id={plot_id}>
        </div>
    }
}
