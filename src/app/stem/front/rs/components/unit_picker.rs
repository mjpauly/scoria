//! Picker for user preference units

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{
    SettingsCard, SettingsCardButtonWithChildren, SettingsCardSelect,
};
use crate::ui_state::FrontState;
use common::units::{LengthUnits, UnitPreference, VelocityUnits};

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
    let check_icon = html! {
        <Icon icon_id={IconId::BootstrapCheck} class="h-6 w-6 text-primary" />
    };
    html! {
        <>

        <SettingsCard class="mb-4">
            <SettingsCardButtonWithChildren onclick={metric_onclick}>
                <label>{"Metric (meters, kilometers)"}</label>
                if *preset == UnitPreset::Metric {
                    { check_icon.clone() }
                }
            </SettingsCardButtonWithChildren>
            <SettingsCardButtonWithChildren onclick={imperial_onclick}>
                <label>{"Imperial (feet, miles)"}</label>
                if *preset == UnitPreset::Imperial {
                    { check_icon.clone() }
                }
            </SettingsCardButtonWithChildren>
            <SettingsCardButtonWithChildren onclick={custom_onclick}>
                <label>{"Custom"}</label>
                if *preset == UnitPreset::Custom {
                    { check_icon }
                }
            </SettingsCardButtonWithChildren>
        </SettingsCard>

        if *preset == UnitPreset::Custom {
            <CustomUnitPicker />
        }

        </>
    }
}

#[function_component]
pub fn CustomUnitPicker() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref.clone());

    let small_length_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_unit: LengthUnits| {
            s.unit_pref.small_length = new_unit;
        },
    );
    let large_length_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_unit: LengthUnits| {
            s.unit_pref.large_length = new_unit;
        },
    );
    let velocity_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_unit: VelocityUnits| {
            s.unit_pref.velocity = new_unit;
        },
    );
    let length_choices =
        LengthUnits::all_variants().cloned().collect::<Vec<_>>();
    let velocity_choices =
        VelocityUnits::all_variants().cloned().collect::<Vec<_>>();
    html! {
        <SettingsCard>
            <SettingsCardSelect<LengthUnits>
                selection={unit_pref.small_length.clone()}
                choices={length_choices.clone()}
                onchange={small_length_onchange}
                text="Small Lengths"
            />
            <SettingsCardSelect<LengthUnits>
                selection={unit_pref.large_length.clone()}
                choices={length_choices}
                onchange={large_length_onchange}
                text="Large Lengths"
            />
            <SettingsCardSelect<VelocityUnits>
                selection={unit_pref.velocity.clone()}
                choices={velocity_choices}
                onchange={velocity_onchange}
                text="Speed"
            />
        </SettingsCard>
    }
}
