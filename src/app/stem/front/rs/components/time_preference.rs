//! Picker for time preferences

use common::units::time::TimeZonePreference;
use jiff::civil::Time;
use strum::IntoEnumIterator;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{
    AfterCardParagraph, SettingsCard, SettingsCardSelect,
    SettingsCardTimeInput, SettingsCardToggle,
};
use crate::ui_state::FrontState;

#[function_component]
pub fn TimeSettings() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let time_pref = use_selector(|s: &FrontState| s.time_pref.clone());

    let twelve_hour_onlick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.time_pref.twelve_hour_clock = !s.time_pref.twelve_hour_clock;
        });

    let day_sep_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_pref: Time| {
            s.time_pref.day_separation_time = new_pref;
        },
    );

    let tz_pref_choices = TimeZonePreference::iter().collect::<Vec<_>>();
    let tz_pref_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_pref: TimeZonePreference| {
            s.time_pref.tz_pref = new_pref;
        },
    );
    let current_tz = jiff::tz::TimeZone::try_system();
    let current_tz_str =
        current_tz.as_ref().map(|tz| tz.iana_name()).ok().flatten();

    let tz_choices = jiff::tz::db().available().collect::<Vec<_>>();
    let tz_choice_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_choice: String| {
            s.time_pref.fixed_tz = new_choice;
        },
    );

    html! {
        <>
            <SettingsCard>
                <SettingsCardToggle
                    checked={time_pref.twelve_hour_clock}
                    onclick={twelve_hour_onlick}
                    text={"Use 12-Hour Clock"}
                />
                <SettingsCardTimeInput
                    text={"Day Separation Time"}
                    value={time_pref.day_separation_time}
                    onchange={day_sep_onchange}
                />
                <SettingsCardSelect<TimeZonePreference>
                    selection={time_pref.tz_pref}
                    choices={tz_pref_choices}
                    onchange={tz_pref_onchange}
                    text="Time Zones"
                    id="time_zone_preference"
                />
                if time_pref.tz_pref == TimeZonePreference::Fixed {
                    <SettingsCardSelect<String>
                        selection={time_pref.fixed_tz.clone()}
                        choices={tz_choices}
                        onchange={tz_choice_onchange}
                        text="Fixed Time Zone"
                        id="fixed_time_zone"
                    />
                }
            </SettingsCard>
            <AfterCardParagraph>
                <p>{"Time Zone Preference"}</p>
                <p class="ml-2">
                    <b>{"Localized: "}</b>
                    {"Display times according to the local time zone rules
                    where data was recorded. For selecting the range of times to
                    view, the time zone at the center of the map is used.
                    (Default)"}
                </p>
                <p class="ml-2">
                    <b>{"Current: "}</b>
                    {"Display all times in your current time zone"}
                    if let Some(tz) = current_tz_str {
                        {", "}<span class="font-mono">{tz}</span>
                    }
                    {"."}
                </p>
                <p class="ml-2">
                    <b>{"Fixed: "}</b>
                    {"Display all times in a fixed time zone."}
                </p>
            </AfterCardParagraph>
        </>
    }
}
