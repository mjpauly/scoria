//! View saved places and change which ones are visible on the map.

use common::pin::Pin;
use common::state::MapSettingsTab;
use yew::prelude::*;
// use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::{BouncyScrollContainerBase, SettingsCard};
use crate::components::{TabBar, TopNav};
use crate::router::Route;
use crate::ui_state::{DerivedState, FrontState};

#[function_component]
pub fn Places() -> Html {
    html! {
        <>
            <TopNav>
                <></>
            </TopNav>
            <BouncyScrollContainerBase class="text-left">
                // <ShowHidePlaces />
                <PlacesList />
            </BouncyScrollContainerBase>
            <TabBar />
        </>
    }
}

// #[function_component]
// pub fn ShowHidePlaces() -> Html {
// html! {
// <div class="px-4">
// </div>
// }
// }

#[function_component]
pub fn PlacesList() -> Html {
    let pins = use_selector(|s: &DerivedState| s.pins.clone());

    let pin_html = pins.iter().map(|p| {
        html! {
            <Place pin={p.clone()} />
        }
    });

    html! {
        <div class="px-4">
            <h1 class="text-primary text-3xl my-6 text-center">
                {"Places"}
            </h1>
            <p class="text-neutral-500 px-2 my-4">
                {"Press and hold on the map to save a place."}
            </p>

            <SettingsCard>
                {for pin_html}
            </SettingsCard>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct PlaceProps {
    #[prop_or_default]
    pub class: Classes,
    pub pin: Pin,
}

const FREEFORM_TEXT_STYLE: &str =
    "whitespace-pre-wrap break-words table table-fixed w-full";

#[function_component]
fn Place(p: &PlaceProps) -> Html {
    let pin = &p.pin;
    let navigator = use_navigator().unwrap();
    let front_dispatch = Dispatch::<FrontState>::new();
    let onclick = {
        let pin = pin.clone();
        front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, _e: MouseEvent| {
                // not currently editing a pin
                state.map.selected_pin_id = pin.id;
                state.map.editable_pin = false;
                state.map.settings_tab = MapSettingsTab::PinDetails;
                state.map.view_pos.center = pin.lnglat;
                state.map.view_pos.zoom = 17.0;
                navigator.push(&Route::Analyze)
            },
        )
    };
    html! {
        <button class="flex items-center justify-between gap-2 \
            text-lg active:bg-neutral-800 px-4 py-2"
            onclick={onclick}
        >
            <span class="whitespace-nowrap"> {pin.icon.clone()} </span>
            <div class="max-h-32 overflow-scroll font-medium text-left">
                <span class={format!("grow {}", FREEFORM_TEXT_STYLE)}>
                    {pin.name.clone()}
                </span>
            </div>
        </button>
    }
}
