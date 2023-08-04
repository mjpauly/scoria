//! Component for configuring location logging settings.

use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{SELECT_STYLE, TOGGLE_SWITCH_STYLE};
use crate::swift_poke;
use crate::ui_state::FrontState;
use common::{LocationAccuracyMode, LocationMode};

static ACCURACY_MODES: [LocationAccuracyMode; 5] = [
    LocationAccuracyMode::Best,
    LocationAccuracyMode::TenMeters,
    LocationAccuracyMode::HundredMeters,
    LocationAccuracyMode::Kilometer,
    LocationAccuracyMode::ThreeKilometers,
];

static LOCATION_MODES: [LocationMode; 4] = [
    LocationMode::Auto,
    LocationMode::Reduced,
    LocationMode::SignificantChanges,
    LocationMode::Standard,
];

/// Configure location manager settings
///
/// Assume these settings are only changed by the user (and not the OS), so we
/// don't listen to backend messages changing the values.
#[function_component]
pub fn LocationConfigurator() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let config = use_selector(|s: &FrontState| s.location_config.clone());

    // convenient aliases for the modes that are selected
    let standard_mode = config.is_standard();

    // enable/disable location
    let enabled_on_click = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.location_config.enabled = !s.location_config.enabled;
            swift_poke::poke(); // notify swift to get new value from backend
        })
    };

    // location mode
    let mode_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_mode = LocationMode::from_str(val).unwrap();
                s.location_config.mode = new_mode;
                swift_poke::poke();
            },
        )
    };
    // accuracy mode
    let accuracy_mode_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_mode = LocationAccuracyMode::from_str(val).unwrap();
                s.location_config.standard_config.accuracy_mode = new_mode;
                swift_poke::poke();
            },
        )
    };
    let mode_options = LOCATION_MODES.iter().map(|x| {
        html! { <option> {x.to_string()} </option> }
    });
    let accuracy_mode_options = ACCURACY_MODES.iter().map(|x| {
        html! { <option> {x.to_string()} </option> }
    });
    // Need to manually set which option is selected in select element in order
    // to do so programmatically (e.g. when we get an updated state)
    let accuracy_select_node_ref = use_node_ref();
    let mode_select_node_ref = use_node_ref();
    {
        let accuracy_select_node_ref = accuracy_select_node_ref.clone();
        let mode_select_node_ref = mode_select_node_ref.clone();
        use_effect_with_deps(
            move |(mode, accurace_mode)| {
                let elem =
                    mode_select_node_ref.cast::<HtmlSelectElement>().unwrap();
                elem.set_value(&(mode.to_string()));
                if standard_mode {
                    let elem = accuracy_select_node_ref
                        .cast::<HtmlSelectElement>()
                        .unwrap();
                    elem.set_value(&(accurace_mode.to_string()));
                }
            },
            // update when these change
            (config.mode.clone(), config.standard_config.accuracy_mode),
        )
    };

    let unit_pref = use_selector(|s: &FrontState| s.unit_pref.clone());
    let dist_filt_text = unit_pref.format_small_length(
        config.standard_config.distance_filter as f64,
        Some(2),
    );
    let dist_filt_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(val) = unit_pref.parse_small_length(&elem.value()) {
                s.location_config.standard_config.distance_filter = val as f32;
                swift_poke::poke();
            }
            elem.set_value("");
        },
    );

    // State for whether we should show help info
    let show_help = use_state(|| false);
    let show_help_onclick = {
        let show_help = show_help.clone();
        Callback::from(move |_e: MouseEvent| show_help.set(!*show_help))
    };

    html! {
        // flex container for centering
        <div class="mt-2 mb-1 flex px-4">
        // centered, width-limited container
        <div class="grow max-w-prose mx-auto">

        // title
        <div class="relative">
            <p class="font-bold"> {"Settings"} </p>
            <button onclick={show_help_onclick} id="loc_conf_help_btn"
                class={"absolute right-2 bottom-0"}>
                    <Icon icon_id={IconId::BootstrapQuestionCircle}
                        class="h-5 w-5 text-neutral-400" />
            </button>
        </div>

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4 mt-2">
            // settings line
            <div class="flex items-center justify-between py-2 \
                border-b border-neutral-800">
                <label for="enable">
                    {"Location Logging"}
                </label>
                // need to wrap the toggle switch with this div or the dot won't
                // scroll with the content
                // h-min wasn't working so height is hardcoded to the switch
                // height of 6
                <div class="relative ml-4 mr-1 h-6">
                <input type="checkbox" id="enable"
                    checked={config.enabled}
                    onclick={enabled_on_click}
                    class={TOGGLE_SWITCH_STYLE} />
                </div>
            </div>
            <div class={format!("flex items-center justify-between py-2 {}",
                if standard_mode {"border-b border-neutral-800" } else {""} )}>
                <label for="location_mode">{"Mode"}</label>
                <select onchange={mode_onchange} id="location_mode"
                    ref={mode_select_node_ref}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for mode_options}
                </select>
            </div>
            if standard_mode {
                <div class="flex items-center justify-between py-2 \
                    border-b border-neutral-800">
                    <label for="accuracy_mode">{"Accuracy"}</label>
                    <select onchange={accuracy_mode_onchange} id="accuracy_mode"
                        ref={accuracy_select_node_ref}
                        class={format!("ml-4 {}", SELECT_STYLE)}>
                        {for accuracy_mode_options}
                    </select>
                </div>
                <div class="flex items-center justify-between py-2">
                    <label for="distance_filter">{"Distance Filter"}</label>
                    <input onchange={dist_filt_onchange}
                        id="distance_filter"
                        placeholder={dist_filt_text}
                        class="ml-4 w-24 rounded bg-black \
                        border border-neutral-700 \
                        placeholder:text-neutral-500" />
                </div>
            }
        </div>

        // help tips
        if *show_help && config.is_auto() {
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Automatic mode continuously records location data, balancing
                battery drain with data accuracy. It logs lower accuracy
                location data while stationary (100 m), and high accuracy data
                while moving (Best)."}
            </p>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"If you want to lower power use, switch to Reduced mode."}
            </p>
        } else if *show_help && config.is_reduced() {
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Reduced mode continuously records location data at a lower
                accuracy level (100 m). It does not switch to a higher accuracy
                mode when moving, so power consumption depends less on how much
                you move."}
            </p>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"If you wish to lower power use, switch to Infrequent mode or
                Custom mode with an accuracy level of 1 km."}
            </p>
        } else if *show_help && standard_mode {
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Custom mode continuously records location data. It gives you
                more control over the location configuration.
                Setting a worse accuracy level (larger distance) sacrifices
                accuracy in exchange for more efficient power use."}
            </p>
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"The distance filter determines how far you must move from
                your last recorded location before recording new data. Set
                it to a larger number to record data less often. 5 meters or
                16 feet is the default."}
            </p>
        } else if *show_help && config.is_infrequent() {
            <p class="text-neutral-500 text-left px-2 pt-1">
                {"Infrequent mode records location only when you move a
                significant distance, like when you visit a new place. It
                saves more power than any of the other modes at the
                cost of a substantially reduced update rate."}
            </p>
        }

        </div>
        </div>
    }
}
