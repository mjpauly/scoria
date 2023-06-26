//! Introduction to Epsilon at startup.

use common::ToBack;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::{
    components::TOGGLE_SWITCH_STYLE, router::Route, swift_poke,
    ui_state::FrontState, websocket::WebsocketService,
};

// Bump to indicate the intro should be shown again to users on install
pub static INTRO_VERSION: usize = 1;

#[function_component]
pub fn Intro() -> Html {
    // record that the intro was viewed
    let dispatch = Dispatch::<FrontState>::new();
    dispatch.reduce_mut(|s| s.last_viewed_intro_version = INTRO_VERSION);

    let navigator = use_navigator().unwrap();
    let exit_intro_onclick =
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Sense));

    // subpage that is being viewed
    let subpage = use_state(|| 1);

    let next_page_onclick = {
        let subpage = subpage.clone();
        Callback::from(move |_e: MouseEvent| {
            subpage.set(*subpage + 1);
        })
    };
    let prev_page_onclick = {
        let subpage = subpage.clone();
        Callback::from(move |_e: MouseEvent| {
            subpage.set(*subpage - 1);
        })
    };

    const LAST_PAGE: usize = 6;
    html! {
        <div class="flex flex-col h-screen">
            <button onclick={exit_intro_onclick} id="exit_intro"
                class="absolute right-6 top-14 p-2 \
                rounded-lg bg-neutral-800 text-neutral-200">
                <Icon icon_id={IconId::BootstrapX} class="h-6 w-6" />
            </button>
            if *subpage > 1 {
                <button onclick={prev_page_onclick}
                    class="absolute left-6 bottom-14 p-2 \
                    rounded-lg bg-neutral-800 text-neutral-200">
                    <Icon icon_id={IconId::BootstrapChevronLeft}
                        class="h-6 w-6" />
                </button>
            }
            if *subpage < LAST_PAGE {
                <button onclick={next_page_onclick}
                    class="absolute right-6 bottom-14 p-2 \
                    rounded-lg bg-neutral-800 text-neutral-200">
                    <Icon icon_id={IconId::BootstrapChevronRight}
                        class="h-6 w-6" />
                </button>
            }
            if *subpage == 1 {
                <TesterNotice />
            } else if *subpage == 2 {
                <BackupNotice />
            } else if *subpage == 3 {
                <IntroStart />
            } else if *subpage == 4 {
                <EnableLocation />
            } else if *subpage == 5 {
                <EnableLocationPartTwo />
            } else if *subpage == 6 {
                <IntroFinish />
            }
        </div>
    }
}

#[function_component]
fn TesterNotice() -> Html {
    html! {
        <div class="my-auto p-5">
            <p class="text-primary text-2xl mb-3">
                {"Thank you for testing Epsilon."}
            </p>
            <p class="mb-3">
                {"Your support and feedback is greatly appreciated. 🙏"}
            </p>
            <p class="mb-3">
                {"The best ways to get in touch are by text or by sending a
                message through the TestFlight app. 🕊️"}
            </p>
        </div>
    }
}

#[function_component]
fn BackupNotice() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let export_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ExportSqliteLog);
        swift_poke::poke();
    });
    html! {
        <div class="my-auto p-5">
            <p class="text-primary text-2xl mb-3">
                {"🛟 Data Backup Recommended"}
            </p>
            <p class="mb-3">
                {"You can now export your location log."}
            </p>
            <p class="mb-3">
                {"If you'd like to reduce the risk of losing your data, we
                recommend that you make a backup. Just tap the button below
                and save your log to the Files app on your device."}
            </p>

            <div class="my-4 flex px-4">
            <div class="grow max-w-prose mx-auto">
            <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
                <button onclick={export_onclick}
                    class="flex items-center justify-between py-2 w-full">
                    <span class="text-primary">
                        {"Export Log"}
                    </span>
                </button>
            </div>
            </div>
            </div>

            <p class="mb-3">
                {"This option is also accessible from the settings."}
            </p>

        </div>
    }
}

#[function_component]
fn IntroStart() -> Html {
    html! {
        <div class="my-auto p-5">
            <p class="text-primary text-2xl mb-3">
                {"Introduction"}
            </p>
            <p class="mb-3">
                {"🧰 Epsilon is your toolkit for privately logging and analyzing
                your location history."}
            </p>
            <p class="mb-3">
                {"🔒 Data is stored only on your phone and is not accessible to
                anyone except you."}
            </p>
        </div>
    }
}

#[function_component]
fn EnableLocation() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let request_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::RequestWhenInUseAuthorization);
        swift_poke::poke();
    });
    html! {
        <div class="my-auto p-5">
            <p class="text-primary text-2xl mb-3">
                {"📍 Enabling Location"}
            </p>
            <p class="mb-3">
                {"Before Epsilon can log your location, you must give it
                permission."}
            </p>
            <p class="mb-3">
                {"When prompted, tap \"Allow While Using App\"."}
            </p>
            <img src="./when_in_use_auth.png" class="w-36 mx-auto mb-3"/>
            <p class="mb-3">
                {"Tap the button to grant access."}
            </p>

            <div class="mt-2 mb-1 flex px-4">
            <div class="grow max-w-prose mx-auto">
            <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
                <button onclick={request_onclick}
                    class="flex items-center justify-between py-2 w-full">
                    <span class="text-primary">
                        {"Enable Location Access"}
                    </span>
                </button>
            </div>
            </div>
            </div>

        </div>
    }
}

#[function_component]
fn EnableLocationPartTwo() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let config = use_selector(|s: &FrontState| s.location_config.clone());
    // enable/disable location
    let enabled_on_click = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.location_config.enabled = !s.location_config.enabled;
            swift_poke::poke(); // notify swift to get new value from backend
        })
    };
    html! {
        <div class="my-auto p-5">
            <p class="mb-3">
                {"To log location data in the background when
                the app is closed, you also need to grant permission to always
                access your location. You will still be able to pause logging at
                any time from within Epsilon."}
            </p>
            <p class="mb-3">
                {"When prompted, tap \"Change to Always Allow\"."}
            </p>
            <img src="./always_auth.png" class="w-36 mx-auto mb-3"/>
            <p class="mb-3">
                {"Tap the slider to grant always access and start logging
                location data."}
            </p>

            <div class="mt-2 mb-1 flex px-4">
            <div class="grow max-w-prose mx-auto">
            <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
                <div class="flex items-center justify-between py-2">
                    <label for="enable"> {"Location Logging"} </label>
                    <div class="relative ml-4 mr-1 h-6">
                    <input type="checkbox" id="enable"
                        checked={config.enabled}
                        onclick={enabled_on_click}
                        class={TOGGLE_SWITCH_STYLE} />
                    </div>
                </div>
            </div>
            </div>
            </div>

        </div>
    }
}

#[function_component]
fn IntroFinish() -> Html {
    html! {
        <div class="my-auto p-5">
            <p class="mb-3">
                {"You can also configure alternate location logging modes
                in the \"Log\" tab."}
            </p>
            <p class="mb-3">
                {"The \"Map\" tab is where you visualize your data."}
            </p>
            <p class="mb-3">
                {"That's it! You're all set up. ✅"}
            </p>
        </div>
    }
}
