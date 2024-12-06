//! View saved places and change which ones are visible on the map.

use common::pin::Pin;
use common::state::MapSettingsTab;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::pin_editor::PinDetailView;
use crate::components::places_filter::PlacesFilterList;
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, SettingsCardSimpleButton,
    TabBar, TopNav,
};
use crate::components::{BouncySavedScrollContainer, SettingsCard};
use crate::router::Route;
use crate::ui_state::{DerivedState, FrontState};

#[function_component]
pub fn Places() -> Html {
    let show_place_detailed_view = use_selector(|s: &FrontState| {
        s.pin_settings.place_detailed_view.is_some()
    });
    html! {
        if *show_place_detailed_view {
            <PlaceDetailedView />
        } else {
            <PlacesList />
        }
    }
}

#[function_component]
pub fn PlacesList() -> Html {
    let pins = use_selector(|s: &DerivedState| s.pins.clone());
    let filters = use_selector(|s: &FrontState| s.pin_settings.filters.clone());

    let pin_html =
        pins.iter().filter(|p| p.passes_filters(&filters)).map(|p| {
            html! {
                <Place pin={p.clone()} />
            }
        });

    let show_filters =
        use_selector(|s: &FrontState| s.pin_settings.show_filters);
    let dispatch = Dispatch::<FrontState>::new();
    let show_filters_onclick =
        dispatch.reduce_mut_callback(|s: &mut FrontState| {
            s.pin_settings.show_filters = !s.pin_settings.show_filters;
        });
    let show_hide_text = if *show_filters {
        "Hide Filters"
    } else {
        "Show Filters"
    };

    html! {
        <>
        <TopNav> <></> </TopNav>
        <BouncySavedScrollContainer class="text-left" id="places-page">
        <div class="px-4 max-w-prose mx-auto">
            <h1 class="text-primary text-3xl my-6 text-center">
                {"Places"}
            </h1>
            <p class="text-neutral-500 px-2 my-4">
                {"Press and hold on the map to save a place."}
            </p>

            <SettingsCard class="mb-4">
                <SettingsCardSimpleButton
                    text={show_hide_text}
                    onclick={show_filters_onclick}
                />
            </SettingsCard>
            <AfterCardParagraph class="mb-4">
                {"Filters determine which places are displayed in the map and
                the timeline. A place is shown if it is true for all active
                filters."}
            </AfterCardParagraph>
        </div>
        <div class="w-screen overflow-x-scroll box-border">
            if *show_filters {
                <PlacesFilterList />
            }
        </div>
        <div class="px-4 mb-6 max-w-prose mx-auto">
            <SettingsCard class="mt-6">
                {for pin_html}
            </SettingsCard>
        </div>
        </BouncySavedScrollContainer>
        <TabBar />
        </>
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
    let front_dispatch = Dispatch::<FrontState>::new();
    let onclick = {
        let pin = pin.clone();
        front_dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.pin_settings.place_detailed_view = Some(pin.clone());
        })
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

/// A detailed view of a place, within the Places tab, that shows past visits to
/// the place.
#[function_component]
fn PlaceDetailedView() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let back_to_list_onclick =
        dispatch.reduce_mut_callback(|s: &mut FrontState| {
            s.pin_settings.place_detailed_view = None;
        });
    let pin = use_selector(|s: &FrontState| {
        s.pin_settings
            .place_detailed_view
            .clone()
            .unwrap_or_default()
    });
    let navigator = use_navigator().unwrap();
    let to_map_onclick = {
        let pin = pin.clone();
        dispatch.reduce_mut_callback_with(
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
        <>
        <TopNav>
            <button class="text-primary flex items-center p-2 px-4"
                onclick={back_to_list_onclick}>
                <Icon icon_id={IconId::BootstrapChevronLeft}
                    class="h-5 w-5" />
                <label>{"All Places"}</label>
            </button>
            <div></div>
        </TopNav>
        <BouncyScrollContainer class="text-left">
            <div class="bg-neutral-900 rounded-lg px-4 py-2 flex flex-col gap-2"
            >
                <PinDetailView pin={(*pin).clone()} />
            </div>
            <SettingsCard class="mt-4">
                <SettingsCardSimpleButton
                    onclick={to_map_onclick}
                    text="Show on Map"
                />
            </SettingsCard>
        </BouncyScrollContainer>
        <TabBar />
        </>
    }
}
