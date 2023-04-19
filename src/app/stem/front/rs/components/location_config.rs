//! Component for configuring location logging settings.

use std::fmt;
use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::common::LocationAccuracyMode;
use crate::components::{SELECT_STYLE, TOGGLE_SWITCH_STYLE};
use crate::swift_poke;
use crate::ui_state::UIState;
use crate::websocket::{ToBack, WebsocketService};

// TODO: switch to using serde for string seralization/deserialization
static ACCURACY_MODE_STRINGS: [(LocationAccuracyMode, &str); 5] = [
    (LocationAccuracyMode::Best, "Best"),
    (LocationAccuracyMode::TenMeters, "10 m"),
    (LocationAccuracyMode::HundredMeters, "100 m"),
    (LocationAccuracyMode::Kilometer, "1 km"),
    (LocationAccuracyMode::ThreeKilometers, "3 km"),
];

impl fmt::Display for LocationAccuracyMode {
    /// Allows us to use `.to_string()`
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = ACCURACY_MODE_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseSelectElemError;

impl std::str::FromStr for LocationAccuracyMode {
    type Err = ParseSelectElemError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = ACCURACY_MODE_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseSelectElemError)?;
        Ok(item.0)
    }
}

/// Configure location manager settings
///
/// Assume these settings are only changed by the user (and not the OS), so we
/// don't listen to backend messages changing the values.
#[function_component]
pub fn LocationConfigurator() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let dispatch = Dispatch::<UIState>::new();
    let location_enabled = use_selector(|s: &UIState| s.location_is_enabled);
    let significant_changes = use_selector(|s: &UIState| s.significant_changes);
    let accuracy_mode = use_selector(|s: &UIState| s.location_accuracy_mode);
    let dist_filt = use_selector(|s: &UIState| s.distance_filter);

    // enable/disable location
    let enabled_on_click = {
        let wss = wss.clone();
        dispatch.reduce_mut_callback(move |s: &mut UIState| {
            s.location_is_enabled = !s.location_is_enabled;
            wss.send_msg(ToBack::SetLocationEnabled(s.location_is_enabled));
            swift_poke::poke(); // notify swift to get new value from backend
        })
    };

    // enable/disable significant changes mode
    let sigchange_on_click = {
        let wss = wss.clone();
        dispatch.reduce_mut_callback(move |s: &mut UIState| {
            s.significant_changes = !s.significant_changes;
            wss.send_msg(ToBack::SetSignificantChanges(s.significant_changes));
            swift_poke::poke();
        })
    };

    // accuracy mode
    let accuracy_mode_onchange = {
        let wss = wss.clone();
        dispatch.reduce_mut_callback_with(move |s: &mut UIState, e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_mode = LocationAccuracyMode::from_str(val).unwrap();
            s.location_accuracy_mode = new_mode;
            wss.send_msg(ToBack::SetLocationAccuracyMode(new_mode));
            swift_poke::poke();
        })
    };
    let accuracy_mode_options = ACCURACY_MODE_STRINGS.iter().map(|x| {
        html! { <option> {x.1.to_string()} </option> }
    });
    // Need to manually set which option is selected in select element in order
    // to do so programmatically (e.g. when we get an updated state)
    let select_node_ref = use_node_ref();
    {
        let select_node_ref = select_node_ref.clone();
        use_effect_with_deps(
            move |mode| {
                let elem = select_node_ref.cast::<HtmlSelectElement>().unwrap();
                elem.set_value(&(mode.to_string()));
            },
            *accuracy_mode, // update whenever accuracy_mode changes
        )
    };

    let dist_filt_onchange =
        dispatch.reduce_mut_callback_with(move |s: &mut UIState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(val) = elem.value().parse::<f32>() {
                s.distance_filter = val;
                wss.send_msg(ToBack::SetDistFilt(val));
                swift_poke::poke();
            }
            elem.set_value("");
        });

    // State for whether we should show help info
    let show_help = use_state(|| false);
    let show_help_onclick = {
        let show_help = show_help.clone();
        Callback::from(move |_e: MouseEvent| show_help.set(!*show_help))
    };

    html! {
        <>
        <div class="relative">
            <p class="mt-6 font-bold"> {"Settings"} </p>
            <button onclick={show_help_onclick} id="loc_conf_help_btn"
                class={"absolute right-6 bottom-0 text-primary"}>
                    {"?"}
            </button>
        </div>
        <div class="mt-2 mb-1 flex px-4">
        <div class="grow max-w-md mx-auto bg-neutral-900 rounded-lg py-1 px-4">
            <div class="flex items-center justify-between py-2 \
                border-b border-neutral-800">
                <label for="location_enabled">
                    {"Standard Location"}
                </label>
                // need to wrap the toggle switch with this div or the dot won't
                // scroll with the content
                <div class="relative ml-4 mr-1 h-6">
                <input type="checkbox" id="location_enabled"
                    checked={*location_enabled} onclick={enabled_on_click}
                    // class={format!("ml-4 mr-1 {}", TOGGLE_SWITCH_STYLE)}
                    class={TOGGLE_SWITCH_STYLE}
                    // disable if significant changes is on and standard
                    // location is off changes is off (this way it can be turned
                    // off if state is bad)
                    disabled={*significant_changes && !*location_enabled} />
                </div>
            </div>
            <div class="flex items-center justify-between py-2 \
                border-b border-neutral-800">
                <label for="accuracy_mode">{"Accuracy"}</label>
                <select onchange={accuracy_mode_onchange} id="accuracy_mode"
                    ref={select_node_ref}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for accuracy_mode_options}
                </select>
            </div>
            <div class="flex items-center justify-between py-2">
                <label for="distance_filter">{"Distance Filter"}</label>
                <div>
                    <input onchange={dist_filt_onchange} id="distance_filter"
                        placeholder={format!("{:.2}", *dist_filt)}
                        class="ml-4 w-16 rounded bg-black \
                        border border-neutral-700 \
                        placeholder:text-neutral-500" />
                    <span class="mx-1">{"m"}</span>
                </div>
            </div>
        </div>
        </div>
        if *show_help {
            <p class="text-neutral-500 text-left px-5">
                {"The standard location service continuously records location.
                Setting a worse accuracy level (larger distance) trades off
                location accuracy for more efficient power use."}
            </p>
            <p class="text-neutral-500 text-left px-5">
                {"The distance
                filter determines how far you must move from your last recorded
                location before recording new data. Set it to a larger number
                to save device storage."}
            </p>
        }
        <div class="mt-2 mb-1 flex px-4">
        <div class="grow max-w-md mx-auto bg-neutral-900 rounded-lg py-1 px-4">
            <div class="flex items-center justify-between py-2">
                <label for="significant_changes">
                    {"Significant Changes"}
                </label>
                <div class="relative ml-4 mr-1 h-6">
                <input type="checkbox" id="significant_changes"
                    checked={*significant_changes} onclick={sigchange_on_click}
                    // class={format!("ml-4 mr-1 {}", TOGGLE_SWITCH_STYLE)}
                    class={TOGGLE_SWITCH_STYLE}
                    // disable if standard location is on and significant
                    // changes is off
                    disabled={*location_enabled && !*significant_changes} />
                </div>
            </div>
        </div>
        </div>
        if *show_help {
            <p class="text-neutral-500 text-left px-5">
                {"The significant location changes service records location
                only when your location changes significantly. It saves more
                power than the standard location service at the cost of
                a significatly reduced update rate."}
            </p>
        }
        </>
    }
}
