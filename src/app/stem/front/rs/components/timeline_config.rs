//! Component for picking the parameters for the timeline

use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::{AfterCardParagraph, SettingsCard, SettingsCardInput};
use crate::ui_state::FrontState;

#[function_component]
pub fn TimelineConfig() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let dwell_duration = use_selector(|s: &FrontState| {
        s.map.timeline_config.long_dwell_width_secs
    });

    let duration_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, new_val: String| {
            if let Ok(val) = new_val.parse::<u64>() {
                s.map.timeline_config.long_dwell_width_secs = val;
            }
        },
    );

    html! {
        <>
            <SettingsCard>
                <SettingsCardInput
                    text="Minimum Dwell Duration"
                    value={dwell_duration.to_string()}
                    onchange={duration_onchange}
                    class="w-16"
                    id="minimum_dwell_duration"
                />
            </SettingsCard>
            <AfterCardParagraph>
                {"The minimum duration in seconds for a dwell to appear in the
                timeline. Also affects the dwell score and long dwell detection
                colormaps. Defaults to 120 seconds."}
            </AfterCardParagraph>
        </>
    }
}
