//! Configuration of sensors, like GPS, audio, etc.

use yew::prelude::*;
// use yew_icons::{Icon, IconId};

use crate::{
    components::NavbarWrapper, event_bus, websocket::WebsocketService,
};

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

            <WssTest />
        </NavbarWrapper>
    }
}

#[function_component]
fn WssTest() -> Html {
    // Get a handle to the websocket service
    let wss = use_context::<WebsocketService>().unwrap();

    // send message to websocket when the button is clicked
    let onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg("clicked".to_string());
    });

    // increment counter on message from websocket
    let counter = use_state(|| 0);
    let on_sock_msg = {
        let counter = counter.clone();
        Callback::from(move |_| {
            counter.set(*counter + 1);
        })
    };
    event_bus::subscribe(on_sock_msg);

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

#[function_component]
fn EnableLocation() -> Html {
    // TODO: get state of location enable/disable from backend at startup
    // Don't need use_state if the backend stores it.
    let location_is_enabled = use_state(|| true);

    let on_click = {
        let location_is_enabled = location_is_enabled.clone();
        Callback::from(move |_e: MouseEvent| {
            let current = *location_is_enabled;
            location_is_enabled.set(!current);
            // TODO: make backend api call to set the state to !current
        })
    };

    html! {
        <>
            <div class="flex justify-center mb-4">
                <input type="checkbox" checked={*location_is_enabled}
                    onclick={on_click} class="mr-4"/>
                if *location_is_enabled {
                    <span class="text-sky-500">{"Enabled"}</span>
                } else {
                    <span class="text-neutral-500">{"Disabled"}</span>
                }
            </div>
            if *location_is_enabled {
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
