//! Page for updating the app (Android only)

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::buttons::{DoneButton, MainSettingsButton};
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, HomeBarSpacer, InfoMessage,
    SettingsCard, SettingsCardExternalLink, TopNav, H1, INFO_BUTTON_BG,
};
use crate::router::SettingsRoute;
use crate::ui_state::{BackState, FrontState};

#[hook]
pub fn use_update_available() -> bool {
    *use_selector(|s: &BackState| s.available_app_version.is_some())
}

#[hook]
pub fn use_show_update_notification() -> bool {
    if !cfg!(feature = "android_config") {
        return false;
    }
    let update_version_code_reviewed =
        use_selector(|s: &FrontState| s.update_version_code_reviewed);
    let available_version =
        use_selector(|s: &BackState| s.available_app_version.clone());
    // Show the notification box only if there is a valid upgrade and it hasn't
    // been dismissed yet.
    match &*available_version {
        Some(avail) => *update_version_code_reviewed < avail.0,
        None => false,
    }
}

#[function_component]
pub fn UpdateNotificationBox() -> Html {
    let available_version =
        use_selector(|s: &BackState| s.available_app_version.clone());
    // Dismissal sets the update_version_code_reviewed to the available version
    // code.
    let dispatch = Dispatch::<FrontState>::new();
    let dismiss_update = {
        let available_version = available_version.clone();
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            if let Some(avail) = &*available_version {
                s.update_version_code_reviewed = avail.0;
            }
        })
    };
    let navigator = use_navigator().unwrap();
    let view_update_page = Callback::from(move |_| {
        navigator.push(&SettingsRoute::Update);
    });
    html! {
        <InfoMessage>
            <div class="flex flex-col items-start pl-2">
                <span class="text-left my-1 font-medium">
                    {"An update for Scoria is available"}
                    if let Some(avail) = &*available_version {
                        {format!(" ({})", avail.1)}
                    }
                </span>
                <div class="flex my-1">
                    <button
                        class={format!("py-2 px-4 text-primary font-semibold \
                        mr-6 rounded-lg {}", INFO_BUTTON_BG)}
                        onclick={dismiss_update}
                    >
                        {"Dismiss"}
                    </button>
                    <button
                        class={format!("py-2 px-4 bg-primary text-white \
                        font-semibold rounded-lg {}", INFO_BUTTON_BG)}
                        onclick={view_update_page}
                    >
                        {"Update"}
                    </button>
                </div>
            </div>
        </InfoMessage>
    }
}

#[function_component]
pub fn Update() -> Html {
    let current_version = use_selector(|s: &BackState| s.app_version.clone());
    let available_version =
        use_selector(|s: &BackState| s.available_app_version.clone());
    let download_button_colors = if available_version.is_some() {
        "bg-primary text-white"
    } else {
        "bg-neutral-800 text-primary"
    };
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Update"} </H1>

                if let Some(avail) = available_version.as_ref() {
                    <p class="mt-2 mx-2 text-left font-medium">
                        {format!("Scoria version {} is available.", avail.1)}
                    </p>
                } else {
                    <p class="mt-2 mx-2 text-left font-medium">
                        {"Scoria is up to date."}
                    </p>
                }
                <p class="mx-2 mt-2 text-left">
                    {format!("Installed: {}", current_version)}
                </p>

                <SettingsCard class="my-4">
                    <SettingsCardExternalLink
                        text="Release Notes"
                        href="https://scoria.info/android/release-notes"
                    />
                </SettingsCard>

                <a class={format!("rounded-lg px-4 py-2 \
                    font-semibold my-6 mx-auto flex w-fit items-center {}",
                    download_button_colors)}
                    href="https://scoria.info/download/apk/latest"
                >
                    {"Download"}
                    <Icon
                        icon_id={IconId::BootstrapDownload}
                        class="ml-4 h-5 w-5"
                    />
                </a>

                <AfterCardParagraph>
                    {"The download will use your default browser app."}
                </AfterCardParagraph>


            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>
    }
}
