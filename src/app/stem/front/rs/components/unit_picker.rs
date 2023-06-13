//! Picker for user preference units

use std::str::FromStr;

use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::{components::SELECT_STYLE, ui_state::FrontState};
use common::units::{
    LengthUnits, VelocityUnits, LENGTH_STRINGS, VELOCITY_STRINGS,
};

// TODO: preset buttons which show as highlighted/selected after click
// ok if this highlighting doesn't persist

#[function_component]
pub fn UnitPicker() -> Html {
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
            unit_pref.clone(),
        )
    };
    html! {
        // flex container for centering
        <div class="mt-2 mb-1 flex px-4">
        // centered, width-limited container
        <div class="grow max-w-prose mx-auto">

        <p class="font-bold"> {"Preferred Units"} </p>

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
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

        </div>
        </div>
    }
}
