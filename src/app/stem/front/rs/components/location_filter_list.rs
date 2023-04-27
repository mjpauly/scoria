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

use std::fmt;
use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::common::Location;
use crate::components::datastream::{DataStream, DATASTREAM_STRINGS};
use crate::components::{SELECT_STYLE, TOGGLE_SWITCH_STYLE};

/// A filter setting. Determines if data points should be excluded based on
/// whether the values in DataStream when compared with the threshold using the
/// operator returns true.
///
/// E.g. (datastream, op, threshold) of:
/// (DataStream::HorizAccuracy, FilterOp::GreaterThan, 20.0)
/// would exclude data where the horizontal accuracy is worse than 20 meters.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub id: usize,
    pub enabled: bool, // quick toggle on/off
    pub datastream: DataStream,
    pub op: FilterOp,
    pub threshold: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterOp {
    GreaterThan,
    LessThan,
    GreatherThanOrEq,
    LessThanOrEq,
    IsEq,
    IsNotEq,
}

impl Filter {
    /// Returns true if we should remove the location and false if we should
    /// keep it.
    pub fn should_remove(&self, loc: &Location) -> bool {
        if !self.enabled {
            // shouldn't remove the location if the filter isn't enabled
            return false;
        }
        let val = self.datastream.get_stream(loc);
        let threshold = self.threshold;
        match self.op {
            FilterOp::GreaterThan => val > threshold,
            FilterOp::LessThan => val < threshold,
            FilterOp::GreatherThanOrEq => val >= threshold,
            FilterOp::LessThanOrEq => val <= threshold,
            FilterOp::IsEq => val == threshold,
            FilterOp::IsNotEq => val != threshold,
        }
    }
}

/// Apply a set of filters to a slice of locations. Data is filtered out if ANY
/// filter condition's `should_remove` method returns true.
///
/// Returns a vector since we need to collect the filter or the closure lives
/// too long.
pub fn apply_filters<'a>(
    filters: &[Filter],
    records: &'a [Location],
) -> Vec<&'a Location> {
    records
        .iter()
        .filter(|loc| {
            let mut should_remove = false;
            for filt in filters {
                should_remove |= filt.should_remove(loc)
            }
            // invert condition, since filter discards on `false`
            !should_remove
        })
        .collect()
}

#[derive(Properties, PartialEq)]
pub struct LocationFilterListProps {
    pub filters: UseStateHandle<Vec<Filter>>,
}

#[function_component]
pub fn LocationFilterList(
    LocationFilterListProps { filters }: &LocationFilterListProps,
) -> Html {
    // global add filter button
    let onremove = {
        let filters = filters.clone();
        Callback::from(move |id: usize| {
            let mut entries = (*filters).clone();
            entries.retain(|entry| entry.id != id);
            filters.set(entries)
        })
    };
    let ontoggle = {
        let filters = filters.clone();
        Callback::from(move |id: usize| {
            let mut entries = (*filters).clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.enabled = !entry.enabled;
            }
            filters.set(entries);
        })
    };
    let onchange_stream = {
        let filters = filters.clone();
        Callback::from(move |(id, datastream): (usize, DataStream)| {
            let mut entries = (*filters).clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.datastream = datastream;
            }
            filters.set(entries);
        })
    };
    let onchange_op = {
        let filters = filters.clone();
        Callback::from(move |(id, op): (usize, FilterOp)| {
            let mut entries = (*filters).clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.op = op;
            }
            filters.set(entries);
        })
    };
    let onchange_threshold = {
        let filters = filters.clone();
        Callback::from(move |(id, threshold): (usize, f64)| {
            let mut entries = (*filters).clone();
            let entry = entries.iter_mut().find(|entry| entry.id == id);
            if let Some(entry) = entry {
                entry.threshold = threshold;
            }
            filters.set(entries);
        })
    };
    let onadd = {
        let filters = filters.clone();
        move |_| {
            let mut entries = (*filters).clone();
            entries.push(Filter {
                id: entries.last().map(|entry| entry.id + 1).unwrap_or(1),
                enabled: false,
                datastream: DataStream::HorizAccuracy,
                op: FilterOp::GreaterThan,
                threshold: 10.0,
            });
            filters.set(entries)
        }
    };
    let num_filters = (**filters).len();
    let height = if num_filters == 0 {
        "h-[4.5rem]" // 18 tailwind units (16 for item + 2x1 margin)
    } else if num_filters == 1 {
        "h-[8.75rem]" // 35 tailwind units (2x16 items + 3x1 margin)
    } else {
        "h-52" // (3x16 items + 4x1 margin)
    };
    html! {
        <>
            <div class={format!("{} overflow-scroll", height)}>
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
                    h-16 m-1">
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
        </>
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
    let onchange_threshold = {
        let onchange_threshold = props.onchange_threshold.clone();
        let stream = filt.datastream.clone();
        move |e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            if let Ok(threshold) = stream.parse_value(val) {
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
            (filt.datastream.clone(), filt.op.clone()),
        )
    };
    html! {
        // horizontal flex
        <div class="flex items-center justify-between py-2 \
            h-16 bg-neutral-900 rounded-lg px-4 m-1">
            <button onclick={onremove} id={format!("filt_{}_remove", filt.id)}>
                <Icon icon_id={IconId::BootstrapXCircle}
                    class="h-5 w-5 text-neutral-500" />
            </button>

            <select class={SELECT_STYLE} ref={stream_node_ref}
                id={format!("filt_{}_datastream", filt.id)}
                    onchange={onchange_stream}>
                {for stream_options.clone()}
            </select>

            <select class={SELECT_STYLE} ref={op_node_ref}
                id={format!("filt_{}_op", filt.id)}
                onchange={onchange_op}>
                {for op_options.clone()}
            </select>

            <input onchange={onchange_threshold}
                id={format!("filt_{}_threshold", filt.id)}
                placeholder={filt.datastream.format_value(filt.threshold)}
                class="w-16 rounded bg-black \
                border border-neutral-700 \
                placeholder:text-neutral-500"
            />

            <div class="relative h-6">
                <input type="checkbox" checked={filt.enabled} onclick={ontoggle}
                    id={format!("filt_{}_toggle", filt.id)}
                    class={TOGGLE_SWITCH_STYLE} />
            </div>

        </div>
    }
}

static OP_STRINGS: [(FilterOp, &str); 6] = [
    (FilterOp::GreaterThan, ">"),
    (FilterOp::LessThan, "<"),
    (FilterOp::GreatherThanOrEq, "≥"),
    (FilterOp::LessThanOrEq, "≤"),
    (FilterOp::IsEq, "="),
    (FilterOp::IsNotEq, "≠"),
];

impl fmt::Display for FilterOp {
    /// Allows us to use `.to_string()` on BasemapStyle
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unwrap since we shouldn't fail to find the enum variant
        let item = OP_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseFilterOpError;

impl std::str::FromStr for FilterOp {
    type Err = ParseFilterOpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = OP_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseFilterOpError)?;
        Ok(item.0.clone())
    }
}
