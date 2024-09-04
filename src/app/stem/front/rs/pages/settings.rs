//! Deeper app settings, such as database exporting
//!
//! Introduction
//!
//! General
//! Map
//! Database
//!
//! Report a problem
//! Privacy

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::map_settings::{
    AutomapSetting, CacheSetting, MiscMapSettings,
};
use crate::components::time_preference::TwelveHourPreference;
use crate::components::timeline_config::TimelineConfig;
use crate::components::unit_picker::UnitPicker;
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, DoneButton, MainSettingsButton,
    SettingsCard, SettingsCardButtonWithChildren, SettingsCardExternalLink,
    SettingsCardPageButton, SettingsCardPageButtonWithLabel,
    SettingsCardSimpleButton, SettingsCardToggle, WarningMessage, H1, H2,
};
use crate::components::{HomeBarSpacer, TopNav};
use crate::pages::update::{
    use_show_update_notification, use_update_available, UpdateNotificationBox,
};
use crate::router::{Route, SettingsRoute};
use crate::swift_poke;
use crate::ui_state::{BackState, FrontState};
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn Settings() -> Html {
    let last_error_reviewed = use_selector(|s: &BackState| {
        s.last_logged_error.as_ref().map(|x| x.1).unwrap_or(true)
    });
    let app_version = use_selector(|s: &BackState| s.app_version.clone());

    let navigator = use_navigator().unwrap();
    let dispatch = Dispatch::<FrontState>::new();
    let to_intro_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.intro_page = 0;
            navigator.push(&Route::Intro);
        });

    let show_update_notification = use_show_update_notification();
    let update_available = use_update_available();
    html! {
        <>
            <TopNav>
                <div></div>
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Settings"} </H1>

                <SettingsCard>
                    <SettingsCardButtonWithChildren onclick={to_intro_onclick}>
                        <label> {"Introduction"} </label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </SettingsCardButtonWithChildren>
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
                        text="Import Places"
                        route={SettingsRoute::Import}
                    />
                    <SettingsCardPageButton<SettingsRoute>
                        text="Export Track"
                        route={SettingsRoute::Export}
                    />
                </SettingsCard>

                <SettingsCard class="my-4">
                    <SettingsCardPageButton<SettingsRoute>
                        text="Database"
                        route={SettingsRoute::Data}
                    />
                </SettingsCard>

                <SettingsCard class="my-4">
                    if cfg!(feature = "android_config") {
                        <SettingsCardPageButtonWithLabel<SettingsRoute>
                            route={SettingsRoute::Update}
                        >
                            <label class="flex items-center">
                                {"Update"}
                                if update_available {
                                    // show icon if there is an update
                                    <Icon
                                        icon_id={IconId::BootstrapInfoCircle}
                                        class="ml-2 h-4 w-4 text-neutral-500"
                                    />
                                }
                            </label>
                        </SettingsCardPageButtonWithLabel<SettingsRoute>>
                    }
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

                <AfterCardParagraph class="mb-4">
                    {"Scoria "}{app_version}
                </AfterCardParagraph>

                if show_update_notification {
                    <UpdateNotificationBox />
                }

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

                <H2> {"Notifications"} </H2>
                <NotificationSettings />

                <H2> {"Units"} </H2>
                <UnitPicker />

                <H2> {"Time"} </H2>
                <TwelveHourPreference />

                <H2> {"Timeline"} </H2>
                <TimelineConfig />
            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}

#[function_component]
pub fn NotificationSettings() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let notif_pref = use_selector(|s: &FrontState| s.notif_pref);
    let should_notify_onclick =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.notif_pref.should_notify_on_stop =
                !s.notif_pref.should_notify_on_stop;
            swift_poke::poke();
        });
    html! {
        <>
            <SettingsCard>
                <SettingsCardToggle
                    checked={notif_pref.should_notify_on_stop}
                    onclick={should_notify_onclick}
                    text={"Notify on Stop"}
                />
            </SettingsCard>
            <AfterCardParagraph>
                {"Notify if Scoria stops logging locations when logging is
                enabled. Might not catch all cases when logging stops."}
            </AfterCardParagraph>
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

                <MiscMapSettings />

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
                <H1> {"Database"} </H1>

                <WarningMessage class="mb-6 mt-2" >
                    {"Your database contains your complete location history.
                    For privacy, avoid sharing it with others."}
                </WarningMessage>

                <SettingsCard>
                    <SettingsCardSimpleButton
                        onclick={export_onclick}
                        text="Export Database"
                    />
                </SettingsCard>
                <AfterCardParagraph>
                    {"An exported database can be imported back into Scoria. You
                    can use this function to backup your data or migrate between
                    devices."}
                </AfterCardParagraph>

                <SettingsCard class="mt-4">
                    <SettingsCardSimpleButton
                        onclick={import_onclick}
                        text="Import Database">
                    </SettingsCardSimpleButton>
                </SettingsCard>
                <AfterCardParagraph>
                    {"Import a Scoria database that was previously exported.
                    The data will be added to your current database. Duplicate
                    data points are determined by timestamp, and are not
                    imported."}
                </AfterCardParagraph>
                <AfterCardParagraph>
                    {"Since imported data is irreversibly added to your
                    database, using this feature for looking at data that is not
                    your own is not recommended."}
                </AfterCardParagraph>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}
