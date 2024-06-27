//! Picker for time preferences

use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{SettingsCard, SettingsCardToggle};
use crate::ui_state::FrontState;

#[function_component]
pub fn TwelveHourPreference() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let time_pref = use_selector(|s: &FrontState| s.time_pref);

    let twelve_hour_onlick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.time_pref.twelve_hour_clock = !s.time_pref.twelve_hour_clock;
        });

    html! {
        <>
            <SettingsCard>
                <SettingsCardToggle
                    checked={time_pref.twelve_hour_clock}
                    onclick={twelve_hour_onlick}
                    text={"Use 12-Hour Clock"}
                />
            </SettingsCard>
        </>
    }
}
