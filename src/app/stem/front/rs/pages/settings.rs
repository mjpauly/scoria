//! Deeper app settings, such as data log exporting

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::components::buttons::DoneButton;
use crate::components::unit_picker::UnitPicker;
use crate::components::{HomeBarSpacer, TopNav};
use crate::router::{Route, SettingsRoute};
use crate::swift_poke;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn Settings() -> Html {
    let navigator = use_navigator().unwrap();
    let intro_onclick = {
        let navigator = navigator.clone();
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Intro))
    };
    let general_onclick = {
        let navigator = navigator.clone();
        Callback::from(move |_e: MouseEvent| {
            navigator.push(&SettingsRoute::General)
        })
    };
    let datalog_onclick = Callback::from(move |_e: MouseEvent| {
        navigator.push(&SettingsRoute::Data)
    });
    html! {
        <>
            <TopNav>
                <div></div>
                <DoneButton />
            </TopNav>

            <div class="grow overflow-scroll h-0 \
                px-4 w-full max-w-prose mx-auto">
                <h1 class="font-bold text-3xl text-left py-4 px-2">
                    {"Settings"}
                </h1>

                // settings card
                <div class="bg-neutral-900 rounded-lg px-4">
                    // settings line
                    <button class="py-2 border-b border-neutral-800 w-full \
                        flex items-center justify-between"
                        onclick={intro_onclick}>
                        <label>{"Introduction"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </button>
                    <button class="py-2 border-b border-neutral-800 w-full \
                        flex items-center justify-between"
                        onclick={general_onclick}>
                        <label>{"General"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </button>
                    <button class="py-2 border-b border-neutral-800 w-full \
                        flex items-center justify-between"
                        onclick={datalog_onclick}>
                        <label>{"Data"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </button>
                    <a class="py-2 border-b border-neutral-800 w-full \
                        flex items-center justify-between"
                        href="https://scoria.info/contact">
                        <label>{"Feedback"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </a>
                    <a class="py-2 w-full \
                        flex items-center justify-between"
                        href="https://scoria.info/privacy">
                        <label>{"Privacy"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </a>
                </div>

            </div>

            <HomeBarSpacer />
        </>
    }
}

#[function_component]
pub fn General() -> Html {
    // let _big_list = (1..41).map(|i| html! { <div>{format!("{}", i)}</div> });
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            // Centered, width-limited content container.
            //
            // "grow overflow-scroll h-0" allow this element to elastically
            // scroll if it's too long to fully show.
            //
            // If we just do "flex-1" instead, we'll get the transparency effect
            // on sticky elements, but the scrolling won't be elastic. We could
            // have bouncy scrolling everywhere, but then that's unnatural for
            // elements that are supposed to by fixed/sticky (though it is the
            // norm for all mobile websites).
            <div class="grow overflow-scroll h-0 \
                px-4 w-full max-w-prose mx-auto">
                <h1 class="font-bold text-3xl text-left py-4 px-2">
                    {"General"}
                </h1>
                <p class="font-bold pb-2 text-left px-4">
                    {"Units"}
                </p>
                <UnitPicker />
                // {for _big_list}
            </div>

            <HomeBarSpacer />
        </>
    }
}

/// Button to place in a TopNav (on the left) which will go to the main settings
/// page.
#[function_component]
pub fn MainSettingsButton() -> Html {
    let navigator = use_navigator().unwrap();
    let main_settings_onclick = Callback::from(move |_e: MouseEvent| {
        navigator.push(&SettingsRoute::Root)
    });
    html! {
        <button class="text-primary flex items-center p-2 px-4"
            onclick={main_settings_onclick}>
            <Icon icon_id={IconId::BootstrapChevronLeft}
                class="h-5 w-5" />
            <label>{"All Settings"}</label>
        </button>
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

            <div class="grow overflow-scroll h-0 \
                px-4 w-full max-w-prose mx-auto">
                <h1 class="font-bold text-3xl text-left px-2 py-4">
                    {"Data"}
                </h1>

                // settings card
                <div class="bg-neutral-900 rounded-lg px-4 mt-2">
                    // settings line
                    <button onclick={export_onclick}
                        class="flex items-center justify-between py-2 w-full">
                        <span class="text-primary">
                            {"Export Log"}
                        </span>
                    </button>
                </div>
                <p class="text-neutral-500 text-left px-2 pt-1">
                    {"Exported logs can be imported back into Scoria. You can
                    use this to backup your data or migrate between devices."}
                </p>

                // settings card
                <div class="bg-neutral-900 rounded-lg px-4 mt-2">
                    // settings line
                    <button onclick={import_onclick}
                        class="flex items-center justify-between py-2 w-full">
                        <span class="text-primary">
                            {"Import Log"}
                        </span>
                    </button>
                </div>
                <p class="text-neutral-500 text-left px-2 pt-1">
                    {"Import a Scoria log that was previously exported.
                    The data will be added to your current log. Duplicate data
                    points are determined by timestamp, and are not imported."}
                </p>
                <p class="text-neutral-500 text-left px-2 pt-1">
                    {"Since imported data is irreversibly added to your log,
                    using this feature for looking at data that is not your own
                    is not recommended. Let us know if you want this kind of
                    feature."} </p>

            </div>

            <HomeBarSpacer />
        </>
    }
}
