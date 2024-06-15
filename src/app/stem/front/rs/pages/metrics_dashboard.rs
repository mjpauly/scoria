//! The dashboard page that shows metadata metrics, such as total distance
//! covered, min/max speeds, and starting and ending times.

use common::map_style::ColoredDataStream;
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
            </BouncyScrollContainerBase>
            <TabBar />
        </>
    }
}

#[function_component]
fn DefaultMetrics() -> Html {
    let metrics = use_selector(|s: &DerivedState| s.dashboard_metrics.clone());

    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);

    let f_count = format!("{} points", metrics.count);
    let unwrap_or_na =
        |maybe_x: Option<String>| maybe_x.unwrap_or_else(|| "N/A".into());
    let f_distance = unwrap_or_na(metrics.total_distance.map(|d| {
        if d > 1000. {
            unit_pref.format_large_length(d, Some(3))
        } else {
            unit_pref.format_small_length(d, Some(3))
        }
    }));
    let f_duration = unwrap_or_na(
        metrics
            .duration
            .map(|x| humantime::format_duration(x.unsigned_abs()).to_string()),
    );
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
                <ScalarMetric title="Count" value={f_count} />
                <ScalarMetric title="Distance" value={f_distance} />
                <ScalarMetric title="Time Span" value={f_duration} />
                <ScalarMetric title="Average Speed" value={f_avg_speed} />
                <ScalarMetric title="Min Speed" value={f_min_speed} />
                <ScalarMetric title="Max Speed" value={f_max_speed} />
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
