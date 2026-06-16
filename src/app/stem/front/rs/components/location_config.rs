//! Component for configuring location logging settings.

use strum::IntoEnumIterator;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{
    AfterCardParagraph, SettingsCard, SettingsCardInput, SettingsCardSelect,
    SettingsCardToggle,
};
use crate::swift_poke;
use crate::ui_state::FrontState;
use common::{LocationAccuracyMode, LocationMode};

/// Configure location manager settings
///
/// Assume these settings are only changed by the user (and not the OS), so we
/// don't listen to backend messages changing the values.
#[function_component]
pub fn LocationConfigurator() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let config = use_selector(|s: &FrontState| s.location_config);

    // convenient aliases for the modes that are selected
    let standard_mode = config.is_standard();

    // enable/disable location
    let enabled_on_click =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.location_config.enabled = !s.location_config.enabled;
            swift_poke::poke(); // notify swift to get new value from backend
        });

    // location mode
    let mode_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_mode: LocationMode| {
            s.location_config.mode = new_mode;
            swift_poke::poke();
        },
    );
    // accuracy mode
    let accuracy_mode_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_mode: LocationAccuracyMode| {
            s.location_config.standard_config.accuracy_mode = new_mode;
            swift_poke::poke();
        },
    );
    let mode_choices = LocationMode::iter().collect::<Vec<_>>();
    #[cfg(not(feature = "android_config"))]
    let accuracy_mode_choices =
        LocationAccuracyMode::iter().collect::<Vec<_>>();
    // Currently the 10m and 3km accuracy modes don't do anything in Android
    #[cfg(feature = "android_config")]
    let accuracy_mode_choices = vec![
        LocationAccuracyMode::Best,
        LocationAccuracyMode::HundredMeters,
        LocationAccuracyMode::Kilometer,
    ];

    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);
    let dist_filt_text = unit_pref.format_small_length(
        config.standard_config.distance_filter as f64,
        Some(2),
    );
    let dist_filt_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_val: String| {
            if let Ok(val) = unit_pref.parse_small_length(&new_val) {
                s.location_config.standard_config.distance_filter = val as f32;
                swift_poke::poke();
            }
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

        <SettingsCard class="mt-2">
            <SettingsCardToggle
                checked={config.enabled}
                onclick={enabled_on_click}
                text="Location Logging"
                id="enable"
            >
            </SettingsCardToggle>
            <SettingsCardSelect<LocationMode>
                selection={config.mode}
                choices={mode_choices}
                onchange={mode_onchange}
                text="Mode"
                id="location_mode"
            />
        </SettingsCard>
        // Separate settings card for Custom/Standard mode settings.
        // (Wrapping SettingsCard lines with `if`s is tricky: an extra div is
        // often created, making the `hr` borders get messed up.)
        if standard_mode {
            <SettingsCard class="mt-2">
                <SettingsCardSelect<LocationAccuracyMode>
                    selection={config.standard_config.accuracy_mode}
                    choices={accuracy_mode_choices}
                    onchange={accuracy_mode_onchange}
                    text="Accuracy"
                    id="accuracy_mode"
                />
                <SettingsCardInput
                    text="Distance Filter"
                    value={dist_filt_text}
                    onchange={dist_filt_onchange}
                    class="w-24"
                    id="distance_filter"
                />
            </SettingsCard>
        }

        // help tips
        if *show_help && config.is_auto() {
            <AfterCardParagraph>
                {"Automatic mode continuously records location data, balancing
                battery drain with data accuracy. It logs lower accuracy
                location data while stationary (100 m), and high accuracy data
                while moving (Best)."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"To lower power use, switch to Reduced mode."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"To keep accuracy and power use high, switch to Custom mode
                with an accuracy of Best. This is useful when low-power location
                services like WiFi are unavailable, and a change from
                stationarity to movement might not be detected."}
            </AfterCardParagraph>
        } else if *show_help && config.is_reduced() {
            <AfterCardParagraph>
                {"Reduced mode continuously records location data at a lower
                accuracy level (100 m). It does not switch to a higher accuracy
                mode when moving, so power consumption depends less on how much
                you move."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"To lower power use, switch to Infrequent mode or Custom mode
                with an accuracy level of 1 km."}
            </AfterCardParagraph>
        } else if *show_help && standard_mode {
            <AfterCardParagraph>
                {"Custom mode continuously records location data. It gives you
                more control over the location configuration.
                Setting a worse accuracy level (larger distance) sacrifices
                accuracy in exchange for more efficient power use."}
            </AfterCardParagraph>
            <AfterCardParagraph>
                {"The distance filter determines how far you must move from
                your last recorded location before recording new data. Set
                it to a larger number to record data less often. 5 meters or
                16 feet is the default."}
            </AfterCardParagraph>
        } else if *show_help && config.is_infrequent() {
            <AfterCardParagraph>
                {"Infrequent mode records locations only when you move a
                significant distance, like when you visit a new place. It
                saves more power than any of the other modes at the
                cost of a substantially reduced update rate."}
            </AfterCardParagraph>
        }

        </div>
        </div>
    }
}
