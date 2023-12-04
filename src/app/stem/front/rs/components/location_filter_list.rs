//! Element for choosing how to filter plotted location data. If any filter's
//! condition is true for a data point, that data point is removed.
//!
//! One filter looks kind of like this:
//! +------------------------------------------------+
//! |                                                |
//! | X     HorizAccuracy      >      20.0     ( O)  |
//! |                                                |
//! +------------------------------------------------+
//! It contains:
//!     - Delete button
//!     - Select for the Datastream to filter on
//!     - Secect for the comparison operator
//!     - Threshold value
//!     - Enable/disable toggle switch

use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::{
    components::{SELECT_STYLE, TOGGLE_SWITCH_STYLE},
    ui_state::FrontState,
};
use common::{
    filters::{DataStream, Filter, FilterOp, DATASTREAM_STRINGS, OP_STRINGS},
    units::LengthUnits,
};

#[function_component]
pub fn LocationFilterList() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let filters = use_selector(|s: &FrontState| s.map.filters.clone());
    // global add filter button
    let onremove = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, id: usize| {
            let mut entries = s.map.filters.clone();
            entries.retain(|entry| entry.id != id);
            s.map.filters = entries
        },
    );
    let ontoggle = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, id: usize| {
            let mut entries = s.map.filters.clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.enabled = !entry.enabled;
            }
            s.map.filters = entries
        },
    );
    let onchange_stream = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, (id, datastream): (usize, DataStream)| {
            let mut entries = s.map.filters.clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.datastream = datastream;
            }
            s.map.filters = entries
        },
    );
    let onchange_op = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, (id, op): (usize, FilterOp)| {
            let mut entries = s.map.filters.clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.op = op;
            }
            s.map.filters = entries
        },
    );
    let onchange_threshold = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, (id, threshold): (usize, f64)| {
            let mut entries = s.map.filters.clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.threshold = threshold;
            }
            s.map.filters = entries
        },
    );
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let onadd =
        dispatch.reduce_mut_callback_with(move |s: &mut FrontState, _| {
            let mut entries = s.map.filters.clone();
            // pick rounder values for imperial units
            let threshold = match unit_pref.small_length {
                LengthUnits::Kilometer => 0.01,
                LengthUnits::Meter => 10.,
                LengthUnits::Foot => LengthUnits::Foot.to_base_unit(30.),
                LengthUnits::Mile => LengthUnits::Mile.to_base_unit(0.006),
                LengthUnits::NauticalMile => {
                    LengthUnits::NauticalMile.to_base_unit(0.005)
                }
            };
            entries.push(Filter {
                id: entries.last().map(|entry| entry.id + 1).unwrap_or(1),
                enabled: false,
                datastream: DataStream::HorizAccuracy,
                op: FilterOp::GreaterThan,
                threshold,
            });
            s.map.filters = entries
        });
    let num_filters = (*filters).len();
    let height = if num_filters == 0 {
        "h-[4.5rem]" // 18 tailwind units (16 for item + 2x1 margin)
    } else if num_filters == 1 {
        "h-[8.75rem]" // 35 tailwind units (2x16 items + 3x1 margin)
    } else {
        "h-52" // (3x16 items + 4x1 margin)
    };
    html! {
        <div class="flex">
            <div class={format!("overflow-scroll max-w-prose grow mx-auto {}",
                                height)}>
                {for (**filters).iter().cloned().map(|filter|
                    html! {
                        <FilterEntry {filter}
                            onremove={onremove.clone()}
                            onchange_stream={onchange_stream.clone()}
                            onchange_op={onchange_op.clone()}
                            onchange_threshold={onchange_threshold.clone()}
                            ontoggle={ontoggle.clone()}
                            />
                    }
                )}
                <div class="flex items-center justify-between pl-4 pr-2
                    h-16 min-h-fit m-1">
                    <p class="text-neutral-500 text-left mr-4">
                        {"Filters hide data where the condition is true. Tap
                            the plus to add another."}
                    </p>
                    <button onclick={onadd} class="p-2">
                        <div class="rounded-lg p-2 bg-neutral-800">
                            <Icon icon_id={IconId::BootstrapPlusLg}
                                class="h-5 w-5 text-neutral-400" />
                        </div>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct FilterEntryProps {
    filter: Filter,
    onremove: Callback<usize>,
    onchange_stream: Callback<(usize, DataStream)>,
    onchange_op: Callback<(usize, FilterOp)>,
    onchange_threshold: Callback<(usize, f64)>,
    ontoggle: Callback<usize>,
}

#[function_component]
fn FilterEntry(props: &FilterEntryProps) -> Html {
    let filt = &props.filter;
    let id = filt.id;

    let onremove = {
        let onremove = props.onremove.clone();
        move |_| onremove.emit(id)
    };
    let ontoggle = {
        let ontoggle = props.ontoggle.clone();
        move |_| ontoggle.emit(id)
    };
    let onchange_stream = {
        let onchange_stream = props.onchange_stream.clone();
        move |e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let stream = DataStream::from_str(val).unwrap();
            onchange_stream.emit((id, stream))
        }
    };
    let onchange_op = {
        let onchange_op = props.onchange_op.clone();
        move |e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let op = FilterOp::from_str(val).unwrap();
            onchange_op.emit((id, op))
        }
    };
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let onchange_threshold = {
        let onchange_threshold = props.onchange_threshold.clone();
        let stream = filt.datastream;
        let unit_pref = unit_pref.clone();
        move |e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            if let Ok(threshold) = stream.parse_value(&unit_pref, val) {
                onchange_threshold.emit((id, threshold))
            }
            elem.set_value("");
        }
    };

    // option html elements for select elements
    let stream_options = DATASTREAM_STRINGS.iter().map(|x| {
        html! { <option> {x.0.to_string()} </option> }
    });
    let op_options = OP_STRINGS.iter().map(|x| {
        html! { <option> {x.1.to_string()} </option> }
    });

    // update the displayed value for select elements
    let stream_node_ref = use_node_ref();
    let op_node_ref = use_node_ref();
    {
        let stream_node_ref = stream_node_ref.clone();
        let op_node_ref = op_node_ref.clone();
        use_effect_with_deps(
            move |(stream, op)| {
                let e = stream_node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(stream.to_string()));
                let e = op_node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(op.to_string()));
            },
            (filt.datastream, filt.op),
        )
    };
    html! {
        // horizontal flex
        <div class="flex flex-wrap items-center justify-between py-1 \
            h-16 min-h-fit bg-neutral-900 rounded-lg px-4 m-1">
            <button onclick={onremove} id={format!("filt_{}_remove", filt.id)}>
                <Icon icon_id={IconId::BootstrapXCircle}
                    class="h-5 w-5 my-1 text-neutral-500" />
            </button>

            <select class={format!("my-1 ml-2 flex-grow {}", SELECT_STYLE)}
                ref={stream_node_ref}
                id={format!("filt_{}_datastream", filt.id)}
                onchange={onchange_stream}>
                {for stream_options.clone()}
            </select>

            <select class={format!("my-1 ml-2 {}", SELECT_STYLE)} ref={op_node_ref}
                id={format!("filt_{}_op", filt.id)}
                onchange={onchange_op}>
                {for op_options.clone()}
            </select>

            <input onchange={onchange_threshold}
                id={format!("filt_{}_threshold", filt.id)}
                placeholder={filt.datastream.format_value(
                                &unit_pref, filt.threshold)}
                class="w-16 flex-grow ml-2 my-1 rounded bg-black \
                border border-neutral-700 \
                placeholder:text-neutral-500"
            />

            <div class="relative h-6 ml-2 my-1">
                <input type="checkbox" checked={filt.enabled} onclick={ontoggle}
                    id={format!("filt_{}_toggle", filt.id)}
                    class={TOGGLE_SWITCH_STYLE} />
            </div>

        </div>
    }
}
