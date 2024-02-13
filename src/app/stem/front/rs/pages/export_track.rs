//! Page for exporting the currently visible track in the map.

use strum::IntoEnumIterator;
use yew::prelude::*;
use yewdux::prelude::*;

use common::export_options::ExportFormat;

use crate::components::buttons::{DoneButton, MainSettingsButton};
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, HomeBarSpacer, SettingsCard,
    SettingsCardInput, SettingsCardSelect, SettingsCardSimpleButton,
    SettingsCardToggle, TopNav, H1,
};
use crate::ui_state::FrontState;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn ExportTrack() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let export_opts = use_selector(|s: &FrontState| s.export_opts);

    let wss = use_context::<WebsocketService>().unwrap();
    let export_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ExportTrack);
    });

    let format_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, f: ExportFormat| s.export_opts.format = f,
    );
    let format_choices = ExportFormat::iter().collect::<Vec<_>>();
    let view_bounded_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.export_opts.view_bounded = !s.export_opts.view_bounded
        });
    let max_points_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, value: String| {
            if let Ok(val) = value.parse() {
                s.export_opts.max_points = val;
            }
        },
    );
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Export Track"} </H1>

                <p class="mt-2 mx-2 text-left">
                    {"Export the data that is visible in the map view, using the
                    selected time range and currently active filters."}
                </p>

                <SettingsCard class="mt-4">
                    <SettingsCardSelect<ExportFormat>
                        text="Format"
                        selection={export_opts.format}
                        choices={format_choices}
                        onchange={format_onchange}
                    />
                    <SettingsCardToggle
                        text="View Bounded"
                        checked={export_opts.view_bounded}
                        onclick={view_bounded_onclick}
                    />
                    <SettingsCardInput
                        text="Maximum Points"
                        value={export_opts.max_points.to_string()}
                        onchange={max_points_onchange}
                        class="w-24"
                    />
                </SettingsCard>

                <AfterCardParagraph>
                    {"View Bounded: Exports only the points that are within the
                    latitude and longitude bounds of the map view. Default:
                    enabled"}
                </AfterCardParagraph>
                <AfterCardParagraph>
                    {"Maximum Points: Caps the number of exported points to
                    remain under the limit. Decimates evenly, selecing every nth
                    point by time. Default: 10,000"}
                </AfterCardParagraph>

                <SettingsCard class="mt-4">
                    <SettingsCardSimpleButton
                        text={format!("Export {}", export_opts.format)}
                        onclick={export_onclick}
                    />
                </SettingsCard>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}
