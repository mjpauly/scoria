//! Deeper app settings, such as data log exporting

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::components::unit_picker::UnitPicker;
use crate::router::{Route, SettingsRoute};
use crate::swift_poke;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn Settings() -> Html {
    let navigator = use_navigator().unwrap();
    let exit_settings_onclick = {
        let navigator = navigator.clone();
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Sense))
    };
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
        <div class="min-h-screen">
            <nav class="sticky top-0 h-24 relative backdrop-blur-xl \
                bg-black/20">
                <div class="flex justify-between bottom-0 absolute pb-1 w-full">
                    <div></div>
                    <button class="text-primary p-2 pr-4"
                        onclick={exit_settings_onclick}>
                        <label>{"Done"}</label>
                    </button>
                </div>
            </nav>
            <div class="px-4 max-w-prose mx-auto">
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
                        href="https://epsln.com/contact">
                        <label>{"Feedback"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </a>
                    <a class="py-2 w-full \
                        flex items-center justify-between"
                        href="https://epsln.com/privacy">
                        <label>{"Privacy"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-4 w-4 text-neutral-500" />
                    </a>
                </div>

            </div>

            // <div class="my-auto">
                // <DataLogSettings />
            // </div>
        </div>
    }
}

#[function_component]
pub fn General() -> Html {
    let navigator = use_navigator().unwrap();
    let main_settings_onclick = {
        let navigator = navigator.clone();
        Callback::from(move |_e: MouseEvent| {
            navigator.push(&SettingsRoute::Root)
        })
    };
    let exit_settings_onclick =
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Sense));
    // let _big_list = (1..41).map(|i| html! { <div>{format!("{}", i)}</div> });
    html! {
        <div class="min-h-screen">
            <nav class="sticky top-0 h-24 relative backdrop-blur-xl \
                bg-black/20">
                <div class="flex justify-between bottom-0 absolute pb-1 w-full">
                    <button class="text-primary flex items-center p-2"
                        onclick={main_settings_onclick}>
                        <Icon icon_id={IconId::BootstrapChevronLeft}
                            class="h-6 w-6" />
                        <label>{"Settings"}</label>
                    </button>
                    <button class="text-primary p-2 pr-4"
                        onclick={exit_settings_onclick}>
                        <label>{"Done"}</label>
                    </button>
                </div>
            </nav>
            // centered, width-limited container
            <div class="px-4 max-w-prose mx-auto">
                <h1 class="font-bold text-3xl text-left py-4 px-2">
                    {"General"}
                </h1>
                <p class="font-bold pb-2 text-left px-4">
                    {"Units"}
                </p>
                <UnitPicker />
                // {for _big_list}
            </div>
        </div>
    }
}

#[function_component]
pub fn DataSettings() -> Html {
    let navigator = use_navigator().unwrap();
    let main_settings_onclick = {
        let navigator = navigator.clone();
        Callback::from(move |_e: MouseEvent| {
            navigator.push(&SettingsRoute::Root)
        })
    };
    let exit_settings_onclick =
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Sense));

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
        <div class="min-h-screen">
            <nav class="sticky top-0 h-24 relative backdrop-blur-xl \
                bg-black/20">
                <div class="flex justify-between bottom-0 absolute pb-1 w-full">
                    <button class="text-primary flex items-center p-2"
                        onclick={main_settings_onclick}>
                        <Icon icon_id={IconId::BootstrapChevronLeft}
                            class="h-6 w-6" />
                        <label>{"Settings"}</label>
                    </button>
                    <button class="text-primary p-2 pr-4"
                        onclick={exit_settings_onclick}>
                        <label>{"Done"}</label>
                    </button>
                </div>
            </nav>

            <div class="px-4 max-w-prose mx-auto">
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
                    {"Exported logs can be imported back into Epsilon. You can
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
                    {"Import an Epsilon log that was previously exported.
                    The data will be added to your current log. Duplicate data
                    points are determined by timestamp, and are not imported."}
                </p>
                <p class="text-neutral-500 text-left px-2 pt-1">
                    {"Since imported data is irreversibly added to your log,
                    using this feature for looking at data that is not your own
                    is not recommended. Let us know if you want this kind of
                    feature."} </p>

            </div>
        </div>
    }
}
