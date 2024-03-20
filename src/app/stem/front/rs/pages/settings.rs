//! Deeper app settings, such as data log exporting
//!
//! Introduction
//!
//! General
//! Map
//! Data Log
//!
//! Report a problem
//! Privacy

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::map_settings::{
    AutomapSetting, CacheSetting, ShowLastLocationSetting,
};
use crate::components::unit_picker::UnitPicker;
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, DoneButton, MainSettingsButton,
    SettingsCard, SettingsCardExternalLink, SettingsCardPageButton,
    SettingsCardPageButtonWithLabel, SettingsCardSimpleButton, WarningMessage,
    H1, H2,
};
use crate::components::{HomeBarSpacer, TopNav};
use crate::router::{Route, SettingsRoute};
use crate::swift_poke;
use crate::ui_state::BackState;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn Settings() -> Html {
    let last_error_reviewed = use_selector(|s: &BackState| {
        s.last_logged_error.as_ref().map(|x| x.1).unwrap_or(true)
    });
    let app_version = use_selector(|s: &BackState| s.app_version.clone());
    html! {
        <>
            <TopNav>
                <div></div>
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Settings"} </H1>

                <SettingsCard>
                    <SettingsCardPageButton<Route>
                        text="Introduction"
                        route={Route::Intro}
                    />
                </SettingsCard>

                <SettingsCard class="my-4">
                    <SettingsCardPageButton<SettingsRoute>
                        text="General"
                        route={SettingsRoute::General}
                    />
                    <SettingsCardPageButton<SettingsRoute>
                        text="Map"
                        route={SettingsRoute::MapSettings}
                    />
                    <SettingsCardPageButton<SettingsRoute>
                        text="Export Track"
                        route={SettingsRoute::Export}
                    />
                    <SettingsCardPageButton<SettingsRoute>
                        text="Data Log"
                        route={SettingsRoute::Data}
                    />
                </SettingsCard>

                <SettingsCard class="my-4">
                    <SettingsCardPageButtonWithLabel<SettingsRoute>
                        route={SettingsRoute::ReportProblem}
                    >
                        <label class="flex items-center">
                            {"Report a Problem"}
                            if !*last_error_reviewed {
                                // show a icon if there is an error to review
                                <Icon
                                    icon_id={IconId::BootstrapInfoCircle}
                                    class="ml-2 h-4 w-4 text-neutral-500"
                                />
                            }
                        </label>
                    </SettingsCardPageButtonWithLabel<SettingsRoute>>
                    <SettingsCardExternalLink
                        text="Privacy"
                        href="https://scoria.info/privacy"
                    />
                </SettingsCard>

                <AfterCardParagraph>
                    {"Scoria v"}{app_version}
                </AfterCardParagraph>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}

#[function_component]
pub fn General() -> Html {
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"General"} </H1>
                <H2> {"Units"} </H2>
                <UnitPicker />
            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}

#[function_component]
pub fn MapSettings() -> Html {
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>
            <BouncyScrollContainer class="pb-8">
                <H1> {"Map"} </H1>

                <ShowLastLocationSetting />

                <H2> {"Automap"} </H2>
                <AutomapSetting />

                <H2> {"Cache"} </H2>
                <CacheSetting />
            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}

#[function_component]
pub fn DataSettings() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let export_onclick = {
        let wss = wss.clone();
        Callback::from(move |_e: MouseEvent| {
            wss.send_msg(ToBack::ExportSqliteLog);
            swift_poke::poke();
        })
    };
    let import_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ImportSqliteLog);
        swift_poke::poke();
    });
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Data Log"} </H1>

                <WarningMessage class="mb-6 mt-2" >
                    {"Your log contains your complete location history.
                    For privacy, avoid sharing it with others."}
                </WarningMessage>

                <SettingsCard>
                    <SettingsCardSimpleButton
                        onclick={export_onclick}
                        text="Export Log"
                    />
                </SettingsCard>
                <AfterCardParagraph>
                    {"An Exported log can be imported back into Scoria. You can
                    use this to backup your data or migrate between devices."}
                </AfterCardParagraph>

                <SettingsCard class="mt-4">
                    <SettingsCardSimpleButton
                        onclick={import_onclick}
                        text="Import Log">
                    </SettingsCardSimpleButton>
                </SettingsCard>
                <AfterCardParagraph>
                    {"Import a Scoria log that was previously exported.
                    The data will be added to your current log. Duplicate data
                    points are determined by timestamp, and are not imported."}
                </AfterCardParagraph>
                <AfterCardParagraph>
                    {"Since imported data is irreversibly added to your log,
                    using this feature for looking at data that is not your own
                    is not recommended. Let us know if you want this kind of
                    feature."}
                </AfterCardParagraph>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}
