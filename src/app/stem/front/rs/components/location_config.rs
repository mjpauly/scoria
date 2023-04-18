//! Component for configuring location logging settings.

use std::fmt;
use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::common::LocationAccuracyMode;
use crate::components::SELECT_STYLE;
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
        html! {
            <option selected={x.0 == *accuracy_mode}>
                {x.1.to_string()}
            </option>
        }
    });

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

    html! {
        <div class="mt-1 mb-1 flex">
        <div class="max-w-fit mx-auto">
            <div class="flex items-center justify-between my-2">
                <label for="location_enabled">
                    {"Standard Location"}
                </label>
                <input type="checkbox" id="location_enabled"
                    checked={*location_enabled} onclick={enabled_on_click}
                    class="ml-8 mr-1" />
            </div>
            <div class="flex items-center justify-between my-2">
                <label for="significant_changes">
                    {"Significant Changes"}
                </label>
                <input type="checkbox" id="significant_changes"
                    checked={*significant_changes} onclick={sigchange_on_click}
                    class="ml-8 mr-1" />
            </div>
            <div class="flex items-center justify-between my-2">
                <label for="accuracy_mode">{"Accuracy"}</label>
                <select onchange={accuracy_mode_onchange} id="accuracy_mode"
                    class={format!("ml-8 {}", SELECT_STYLE)}>
                    {for accuracy_mode_options}
                </select>
            </div>
            <div class="flex items-center justify-between my-2">
                <label for="distance_filter">{"Distance Filter"}</label>
                <input onchange={dist_filt_onchange} id="distance_filter"
                    placeholder={format!("{:.2}", *dist_filt)}
                    class="ml-8 w-16 rounded bg-black \
                    border border-neutral-700 \
                    placeholder:text-neutral-500" />
                <span class="mx-1">{"m"}</span>
            </div>
        </div>
        </div>
    }
}
