//! Configuration of sensors, like GPS, audio, etc.

use yew::prelude::*;
// use yew_icons::{Icon, IconId};

use crate::components::NavbarWrapper;
use crate::ui_state::UIState;
use crate::websocket::{ToBack, ToFront, WebsocketService};

#[function_component]
pub fn Sense() -> Html {
    html! {
        <>
            <LocationConfig />
        </>
    }
}

#[function_component]
fn LocationConfig() -> Html {
    html! {
        <NavbarWrapper>
            // <Icon icon_id={IconId::BootstrapSoundwave} />
            <h1 class="text-sky-500 text-3xl mb-6">
                {"Location"}
            </h1>

            <EnableLocation />

            <button class="rounded-lg whitespace-nowrap \
                    py-1.5 px-3 text-sky-500 bg-neutral-800">
                {"Share SQLite log"}
            </button>

            // <WssTest />
        </NavbarWrapper>
    }
}

/*
#[function_component]
fn WssTest() -> Html {
    // Get a handle to the websocket service
    let wss = use_context::<WebsocketService>().unwrap();
    let wss1 = wss.clone();

    // send message to websocket when the button is clicked
    let onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::GetLocationEnabled);
    });

    // set counter on message from websocket
    let counter = use_state(|| 0);
    let on_sock_msg = {
        let counter = counter.clone();
        move |msg: &ToFront| {
            if let ToFront::Data(val) = msg {
                counter.set(*val);
            }
        }
    };
    wss1.subscribe(Box::new(on_sock_msg));
    html! {
        <>
            <br />
            <button {onclick} class="rounded-lg whitespace-nowrap \
                    py-1.5 px-3 text-sky-500 bg-neutral-800 mt-6">
                {"Send click to websocket"}
            </button>

            <br />
            <p>{ *counter }</p>
        </>
    }
}
*/

#[function_component]
fn EnableLocation() -> Html {
    // Get a clone of the websocket service so we can TX/RX backend messages
    let wss = use_context::<WebsocketService>().unwrap();
    // Get a clone of the app state, so we initialize our local state correctly
    let state = use_context::<UIState>().unwrap();

    // Get the location_is_enabled state from the UI's local copy
    let location_is_enabled = *state.location_is_enabled.borrow();

    // subscribe to backend updates to the location enabled state
    let on_backend_msg = {
        let trigger = use_force_update();
        move |msg: &ToFront| {
            if let ToFront::LocationEnabled(_) = msg {
                // let the rerender show the correct stuff
                trigger.force_update();
            }
        }
    };
    wss.subscribe(Box::new(on_backend_msg));

    // tell backend when we've toggled the location enable state
    let on_click = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::SetLocationEnabled(!location_is_enabled));
    });

    html! {
        <>
            <div class="flex justify-center mb-4">
                <input type="checkbox" checked={location_is_enabled}
                    onclick={on_click} class="mr-4"/>
                if location_is_enabled {
                    <span class="text-sky-500">{"Enabled"}</span>
                } else {
                    <span class="text-neutral-500">{"Disabled"}</span>
                }
            </div>
            if location_is_enabled {
                <LocationDetails />
            }
        </>
    }
}

#[function_component]
fn LocationDetails() -> Html {
    html! {
        <>
            <p class="font-bold">
                {"Current Location:"}
            </p>
            <p class="mb-4">
                {"xxx, yyy"}
            </p>

            <p class="whitespace-nowrap">
                {"Num data points in past hour: xxx"}
            </p>
            <p class="mb-4">
                {"xxx updates/minute"}
            </p>

            <div class="flex justify-center mb-4">
                <p class="mr-6">
                    {"Distance Filter"}
                </p>
                <input id="dist_filt" class="w-12 rounded bg-neutral-900 \
                        border border-neutral-700" />
                <p class="text-sky-500 ml-2">
                    {"?"}
                </p>
            </div>
        </>
    }
}
