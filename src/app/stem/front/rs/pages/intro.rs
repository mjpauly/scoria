//! Introduction to Scoria at startup.

use std::str::FromStr;

use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{BouncyScrollContainer, SELECT_STYLE};
use crate::{
    components::{
        buttons::DoneButton, unit_picker::UnitPicker, BottomNav, TopNav,
        TOGGLE_SWITCH_STYLE,
    },
    swift_poke,
    ui_state::FrontState,
    websocket::WebsocketService,
};
use common::LocationMode;
use common::ToBack;

// Bump to indicate the intro should be shown again to users on install
pub static INTRO_VERSION: u32 = 3;

#[cfg(not(any(feature = "ios_config", feature = "android_config")))]
compile_error!(
    "Either feature \"ios_config\" or \"android_config\" must be enabled."
);

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

    const LAST_PAGE: u32 = 9;
    html! {
        <>
            <TopNav>
                <div></div>
                <DoneButton />
            </TopNav>
            <BouncyScrollContainer class="flex flex-col">
                <div class="my-auto py-5">
                    if *subpage == 1 {
                        <IntroStart />
                    } else if *subpage == 2 {
                        <HowItWorks />
                    } else if *subpage == 3 {
                        <HoldUp />
                    } else if *subpage == 4 {
                        <SeePrivacyPolicy />
                    } else if *subpage == 5 {
                        <LoggingMode />
                    } else if *subpage == 6 {
                        <EnableLocation />
                    } else if *subpage == 7 {
                        <EnableLocationPartTwo />
                    } else if *subpage == 8 {
                        <PickUnits />
                    } else if *subpage == 9 {
                        <IntroFinish />
                    }
                </div>
            </BouncyScrollContainer>
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
fn BackupNotice() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let export_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ExportSqliteLog);
        swift_poke::poke();
    });
    html! {
        <>
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

        </>
    }
}

#[function_component]
fn IntroStart() -> Html {
    html! {
        <>
            <p class="text-primary text-2xl mb-6">
                {"Introduction"}
            </p>
            <p class="mb-6 italic">
                {"Updated Aug 2, 2023"}
            </p>
            <p class="mb-3">
                {"🗺️ Scoria is your toolkit for privately logging and analyzing
                your location history."}
            </p>
            <p class="mb-3">
                {"🔒 Data is stored only on your device and is not accessible to
                anyone except you."}
            </p>
        </>
    }
}

#[function_component]
fn HowItWorks() -> Html {
    html! {
        <>
            <p class="text-primary text-2xl mb-6">
                {"How it Works"}
            </p>
            <p class="mb-3">
                {"📱 Scoria collects your movement history by logging your location while it's open in the background. It's designed so that you can leave it on all the time."}
            </p>
            <p class="mb-3">
                {"🔋 By default, battery drain is minimized by lowering data accuracy when the app detects that you are stationary."}
            </p>
            <p class="mb-3">
                {"🛤️ You can see where you've been, the routes you've taken, time spent at destinations and during travel, and much more. Scoria is an automatic spatial activity journal."}
            </p>
        </>
    }
}

#[function_component]
fn HoldUp() -> Html {
    html! {
        <>
            <p class="text-primary text-2xl mb-6">
                {"✋ Hold Up"}
            </p>
            <p class="mb-3">
                {"Let's take a moment to appreciate what we're talking about here. Scoria is designed to help you record detailed location data about your life, "} <span class="italic">{"continuously."}</span>
            </p>
            <p class="mb-3">
                {"This information is deeply personal, private, and sensitive. It reveals a tremendous amount about who you are. We, the developers, do not take this lightly. We created Scoria because we believe everyone deserves privacy, and because we felt there were gaps in the space of personal data tools."}
            </p>
            <p class="mb-3">
                {"Data logged by the app is kept on your device and is only accessible to you. We don't automatically collect any information about you or how you use the app. We make no assumptions about what data is sensitive and what data isn't. We do not know if you use the app or not, how much you use the app, or if the app crashes while you're using it. (If it crashes, please consider letting us know via the feedback form so we can fix it.) All data is private by default."}
            </p>

        </>
    }
}

#[function_component]
fn SeePrivacyPolicy() -> Html {
    html! {
        <>
            <p class="text-primary text-2xl mb-6">
                {"The Privacy Policy"}
            </p>
            <p class="mb-3">
                {"Our privacy policy is meant to be short and easy to read, and goes into a little more detail than we're dedicating space for in this introduction."}
            </p>
            <p class="mb-3">
                {"Check it out "}
                <a href="https://scoria.info/privacy" class="underline text-blue-500">
                    {"here"}
                </a>
                {"."}
            </p>
            <p class="mb-3">
                {"You can find it anytime from the Settings menu."}
            </p>

        </>
    }
}

static LOCATION_MODES: [LocationMode; 3] = [
    LocationMode::Auto,
    LocationMode::Reduced,
    LocationMode::SignificantChanges,
];

#[function_component]
fn LoggingMode() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let config = use_selector(|s: &FrontState| s.location_config);

    // location mode
    let mode_onchange = {
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
                let val: &str = &elem.value();
                let new_mode = LocationMode::from_str(val).unwrap();
                s.location_config.mode = new_mode;
                swift_poke::poke();
            },
        )
    };

    let mode_options = LOCATION_MODES.iter().map(|x| {
        html! { <option> {x.to_string()} </option> }
    });

    // Need to manually set which option is selected in select element in order
    // to do so programmatically (e.g. when we get an updated state)
    let mode_select_node_ref = use_node_ref();
    {
        let mode_select_node_ref = mode_select_node_ref.clone();
        use_effect_with_deps(
            move |mode| {
                let elem =
                    mode_select_node_ref.cast::<HtmlSelectElement>().unwrap();
                elem.set_value(&(mode.to_string()));
            },
            // update when these change
            config.mode,
        )
    };
    html! {
        <>
            <p class="text-primary text-2xl mb-6">
                {"Location Logging Mode"}
            </p>
            <p class="mb-3">
                {"⚡️ Your device's location sensors take power to operate. This power use depends on the accuracy of the data you want to acquire, your device model, and how much time you spend in motion."}
            </p>
            <p class="mb-3">
                {"☀️ \"Automatic\" mode is the default choice for collecting high accuracy data and making detailed visualizations of your movement. Power is conserved when you're stationary."}
            </p>
            <p class="mb-3">
                {"⛅️ \"Reduced\" mode is a good choice if you want a lower level of power draw and don't mind lower accuracy data, have a device model that's a few years old, or spend most of the day in motion without the ability to recharge your device."}
            </p>
            <p class="mb-3">
                {"🌧️ Choose \"Infrequent\" mode if battery life is critical. Data is logged very rarely with this mode, and is the least detailed. Battery drain is negligible."}
            </p>

            <div class={"bg-neutral-900 rounded-lg px-4 mt-2 flex items-center justify-between py-2"}>
                <label for="location_mode">{"Mode"}</label>
                <select onchange={mode_onchange} id="location_mode"
                    ref={mode_select_node_ref}
                    class={format!("ml-4 {}", SELECT_STYLE)}>
                    {for mode_options}
                </select>
            </div>
        </>
    }
}

#[function_component]
fn EnableLocation() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let request_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::RequestWhenInUseAuthorization);
        swift_poke::poke();
    });
    #[cfg(feature = "ios_config")]
    let tap_prompt = "When prompted, tap \"Allow While Using App\".";
    #[cfg(feature = "android_config")]
    let tap_prompt = "When prompted, tap \"While using the app\".";
    html! {
        <>
            <p class="text-primary text-2xl mb-3">
                {"📍 Enabling Location"}
            </p>
            <p class="mb-3">
                {"Before Scoria can log your location, you must give it
                permission."}
            </p>
            <p class="mb-3">
                {tap_prompt}
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

        </>
    }
}

#[function_component]
fn EnableLocationPartTwo() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let config = use_selector(|s: &FrontState| s.location_config);
    // enable/disable location
    let enabled_on_click = {
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.location_config.enabled = !s.location_config.enabled;
            swift_poke::poke(); // notify swift to get new value from backend
        })
    };
    html! {
        <>
            <EnableLocationPartTwoHelper />

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

        </>
    }
}

/// iOS-specific text for the second EnableLocation page.
#[cfg(feature = "ios_config")]
#[function_component]
fn EnableLocationPartTwoHelper() -> Html {
    html! {
        <>
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

        </>
    }
}

/// Android-specific text for the second EnableLocation page.
#[cfg(feature = "android_config")]
#[function_component]
fn EnableLocationPartTwoHelper() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let go_to_settings_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::GoToLocationSettings);
        swift_poke::poke();
    });
    html! {
        <>
        <p class="mb-1">
            {"You may need to enable low-power location modes in your phone's
            settings. Go to"}
        </p>
        <p class="font-mono mb-1">
            {"Settings > Location > Location services > Google Location
            Accuracy"}
        </p>
        <p class="mb-1">
            {"and enable"}
        </p>
        <p class="font-mono mb-3">
            {"Improve Location Accuracy"}
        </p>
        <div class="mt-2 mb-3 flex px-4">
        <div class="grow max-w-prose mx-auto">
        <div class="bg-neutral-900 rounded-lg px-4 py-1">
            <button onclick={go_to_settings_onclick}
                class="flex items-center justify-between py-2 w-full">
                <span class="text-primary">
                    {"Open Location Settings"}
                </span>
            </button>
        </div>
        </div>
        </div>
        <p class="mb-8 text-sm">
            {"Without this, Scoria can still use high power location modes, but
            you should watch your battery and pause logging when you want to
            save power. Use \"Custom\" mode with an accuracy of \"Best\"."}
        </p>

        <p class="mb-3">
            {"Tap the slider to start logging location data. You will be able to
            pause logging at any time in the app."}
        </p>
        </>
    }
}

#[function_component]
fn PickUnits() -> Html {
    html! {
        <>
            <p class="mb-3">
                {"Finally, select your preferred system of units:"}
            </p>
            <UnitPicker />
        </>
    }
}

#[function_component]
fn IntroFinish() -> Html {
    html! {
        <>
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
        </>
    }
}
