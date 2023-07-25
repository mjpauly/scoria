//! Introduction to Epsilon at startup.

use common::ToBack;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::{
    components::{
        buttons::DoneButton, unit_picker::UnitPicker, BottomNav, TopNav,
        TOGGLE_SWITCH_STYLE,
    },
    swift_poke,
    ui_state::FrontState,
    websocket::WebsocketService,
};

// Bump to indicate the intro should be shown again to users on install
pub static INTRO_VERSION: usize = 2;

#[function_component]
pub fn Intro() -> Html {
    // record that the intro was viewed
    let dispatch = Dispatch::<FrontState>::new();
    dispatch.reduce_mut(|s| s.last_viewed_intro_version = INTRO_VERSION);

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
        <>
            <TopNav>
                <div></div>
                <DoneButton />
            </TopNav>
            <div class="grow overflow-scroll h-0 w-full \
                flex flex-col">
                // if *subpage == 1 {
                    // <TesterNotice />
                // } else if *subpage == 2 {
                    // <BackupNotice />
                if *subpage == 1 {
                    <IntroStart />
                } else if *subpage == 2 {
                    <HowItWorks />
                } else if *subpage == 3 {
                    <EnableLocation />
                } else if *subpage == 4 {
                    <EnableLocationPartTwo />
                } else if *subpage == 5 {
                    <PickUnits />
                } else if *subpage == 6 {
                    <IntroFinish />
                }
            </div>
            <BottomNav>
                if *subpage > 1 {
                    <button onclick={prev_page_onclick}
                        class="text-primary flex items-center p-2 px-4">
                        <Icon icon_id={IconId::BootstrapChevronLeft}
                            class="h-6 w-6" />
                        <label>{"Previous"}</label>
                    </button>
                } else {
                    <div></div>
                }
                if *subpage < LAST_PAGE {
                    <button onclick={next_page_onclick}
                        class="text-primary flex items-center p-2 px-4">
                        <label>{"Next"}</label>
                        <Icon icon_id={IconId::BootstrapChevronRight}
                            class="h-6 w-6" />
                    </button>
                } else {
                    <div></div>
                }
            </BottomNav>
        </>
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
            <p class="text-primary text-2xl mb-6">
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
            <p class="mt-12">
                {"For more about how Epsilon protects your privacy, see the "}
                <a href="https://epsln.com/privacy" class="underline text-blue-500">
                    {"privacy policy"}
                </a>
                {"."}
            </p>
        </div>
    }
}

#[function_component]
fn HowItWorks() -> Html {
    html! {
        <div class="my-auto p-5">
            <p class="text-primary text-2xl mb-6">
                {"How it Works"}
            </p>
            <p class="mb-3">
                {"🗺️ Epsilon lets you record and analyze your movement
                history. You can see where you've been, the routes you've taken,
                the time spent for each section of travel, and more."}
            </p>
            <p class="mb-3">
                {"📱 Epsilon collects your movement history by logging your
                location while the app is open in the background. It's designed
                so that you can leave it on all the time."}
            </p>
            <p class="mb-3">
                {"🔋 Battery drain is minimized by lowering data
                accuracy when the app detects that you are stationary.
                If you want to further conserve battery charge, you
                can switch to a lower accuracy mode that consumes less power
                but still logs what destinations you visited."} 
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
                access your location. You will be able to pause logging at
                any time in the app."}
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
fn PickUnits() -> Html {
    html! {
        <div class="p-5 my-auto">
            <p class="mb-3">
                {"Finally, select your preferred system of units:"}
            </p>
            <UnitPicker />
        </div>
    }
}

#[function_component]
fn IntroFinish() -> Html {
    html! {
        <div class="my-auto p-5">
            <p class="mb-3">
                {"That's it! You're all set up. ✅"}
            </p>
            <p class="mb-3">
                {"Once you've collected some movement data, you can visualize it
                in the \"Map\" tab."}
            </p>
            <p class="mb-3">
                {"Close this page to finish the introduction."}
            </p>
        </div>
    }
}
