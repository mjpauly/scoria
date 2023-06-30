//! Picker for user preference units

use std::str::FromStr;

use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::{components::SELECT_STYLE, ui_state::FrontState};
use common::units::{
    LengthUnits, UnitPreference, VelocityUnits, LENGTH_STRINGS,
    VELOCITY_STRINGS,
};

/// Show three preset options to the user when the open the page. If their
/// settings aren't exactly the same as a the metric or imperial defaults, or
/// they select "Custom", show the CustomUnitPicker element.
#[derive(PartialEq)]
enum UnitPreset {
    Metric,
    Imperial,
    Custom,
}

#[function_component]
pub fn UnitPicker() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref.clone());

    // show the correct preset as selected if the units match
    let preset = use_state(|| {
        if *unit_pref == UnitPreference::metric_default() {
            UnitPreset::Metric
        } else if *unit_pref == UnitPreference::imperial_default() {
            UnitPreset::Imperial
        } else {
            UnitPreset::Custom
        }
    });
    let metric_onclick = {
        let preset = preset.clone();
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            preset.set(UnitPreset::Metric);
            s.unit_pref = UnitPreference::metric_default();
        })
    };
    let imperial_onclick = {
        let preset = preset.clone();
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            preset.set(UnitPreset::Imperial);
            s.unit_pref = UnitPreference::imperial_default();
        })
    };
    let custom_onclick = {
        let preset = preset.clone();
        Callback::from(move |_e| {
            preset.set(UnitPreset::Custom);
        })
    };
    html! {
        <>

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4">
            // settings line
            <button class="py-2 border-b border-neutral-800 w-full \
                flex items-center justify-between" onclick={metric_onclick}>
                <label>{"Metric (meters, kilometers)"}</label>
                if *preset == UnitPreset::Metric {
                    <Icon icon_id={IconId::BootstrapCheck}
                        class="h-6 w-6 text-primary" />
                }
            </button>
            <button class="py-2 border-b border-neutral-800 w-full \
                flex items-center justify-between" onclick={imperial_onclick}>
                <label>{"Imperial (feet, miles)"}</label>
                if *preset == UnitPreset::Imperial {
                    <Icon icon_id={IconId::BootstrapCheck}
                        class="h-6 w-6 text-primary" />
                }
            </button>
            <button class="py-2 w-full flex items-center justify-between"
                onclick={custom_onclick}>
                <label>{"Custom"}</label>
                if *preset == UnitPreset::Custom {
                    <Icon icon_id={IconId::BootstrapCheck}
                        class="h-6 w-6 text-primary" />
                }
            </button>
        </div>

        if *preset == UnitPreset::Custom {
            <div class="h-4"></div>
            <CustomUnitPicker />
        }

        </>
    }
}

#[function_component]
pub fn CustomUnitPicker() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref.clone());

    let large_length_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_unit = LengthUnits::from_str(val).unwrap();
                s.unit_pref.large_length = new_unit;
            },
        )
    };
    let small_length_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_unit = LengthUnits::from_str(val).unwrap();
                s.unit_pref.small_length = new_unit;
            },
        )
    };
    let velocity_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_unit = VelocityUnits::from_str(val).unwrap();
                s.unit_pref.velocity = new_unit;
            },
        )
    };

    let large_length_options = LENGTH_STRINGS.iter().map(|x| {
        html! { <option> {x.0.to_string()} </option> }
    });
    let small_length_options = LENGTH_STRINGS.iter().map(|x| {
        html! { <option> {x.0.to_string()} </option> }
    });
    let velocity_options = VELOCITY_STRINGS.iter().map(|x| {
        html! { <option> {x.0.to_string()} </option> }
    });

    let large_length_node_ref = use_node_ref();
    let small_length_node_ref = use_node_ref();
    let velocity_node_ref = use_node_ref();
    {
        let large_length_node_ref = large_length_node_ref.clone();
        let small_length_node_ref = small_length_node_ref.clone();
        let velocity_node_ref = velocity_node_ref.clone();
        // need to manually set select element value after render
        use_effect_with_deps(
            move |unit_pref| {
                let e =
                    large_length_node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(unit_pref.large_length.to_string()));
                let e =
                    small_length_node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(unit_pref.small_length.to_string()));
                let e = velocity_node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(unit_pref.velocity.to_string()));
            },
            unit_pref,
        )
    };
    html! {
        // settings card
        <div class="bg-neutral-900 rounded-lg px-4">
            // settings line
            <div class="flex items-center justify-between py-2 \
                border-b border-neutral-800">
                <label for="small_length">{"Small Lengths"}</label>
                <select ref={small_length_node_ref} id="small_length"
                    onchange={small_length_onchange}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for small_length_options.clone()}
                </select>
            </div>
            <div class="flex items-center justify-between py-2 \
                border-b border-neutral-800">
                <label for="large_length">{"Large Lengths"}</label>
                <select ref={large_length_node_ref} id="large_length"
                    onchange={large_length_onchange}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for large_length_options.clone()}
                </select>
            </div>
            <div class="flex items-center justify-between py-2">
                <label for="velocity">{"Speed"}</label>
                <select ref={velocity_node_ref} id="velocity"
                    onchange={velocity_onchange}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for velocity_options.clone()}
                </select>
            </div>
        </div>
    }
}
