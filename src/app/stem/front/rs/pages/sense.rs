//! Configuration of sensors, like GPS, audio, etc.

// Profile how long things take with:
// log::debug!(
// "refreshing at {:?}",
// web_sys::window().unwrap().performance().unwrap().now() as i64 % 1000
// );

use web_sys::HtmlInputElement;
use yew::prelude::*;
// use yew_icons::{Icon, IconId};

use crate::components::NavbarWrapper;
use crate::swift_poke;
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

            // <EnableLocation />
            <LocationDetails />

            // <WssTest />
        </NavbarWrapper>
    }
}

// Alternate way to update if we only wanted to update data from the backend
// let trigger = use_force_update();
// trigger.force_update();
#[function_component]
fn EnableLocation() -> Html {
    // Get a clone of the websocket service so we can TX/RX backend messages
    let wss = use_context::<WebsocketService>().unwrap();
    // Get a clone of the app state, so we initialize our local state correctly
    let state = use_context::<UIState>().unwrap();

    // Get the location_is_enabled state from the UI's local copy.
    // We store it in use_state_eq so a click on the checkbox can immediately
    // refresh this component, and if we get an update from the backend it is
    // only refreshed if the new state is different.
    let location_is_enabled =
        use_state_eq(|| *state.location_is_enabled.borrow());

    // Subscribe to backend updates to the location enabled state.
    // Only needed if we expect the backend to change this state without user
    // input.
    let on_backend_msg = {
        let location_is_enabled = location_is_enabled.clone();
        move |msg: &ToFront| {
            if let ToFront::LocationEnabled(val) = msg {
                location_is_enabled.set(*val);
            }
        }
    };
    // Generate a unique ID which doesn't change between renders since no deps
    // are given to use_memo
    let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    wss.subscribe(*id, Box::new(on_backend_msg));

    // Update our state on click and tell the backend. Telling the backend is
    // only needed if the backend needs to know about this state change.
    let on_click = {
        let location_is_enabled = location_is_enabled.clone();
        Callback::from(move |_e: MouseEvent| {
            let new_val = !*location_is_enabled;
            location_is_enabled.set(new_val);
            wss.send_msg(ToBack::SetLocationEnabled(new_val));
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
    let wss = use_context::<WebsocketService>().unwrap();
    let state = use_context::<UIState>().unwrap();
    let last_loc = use_state_eq(|| state.last_location.borrow().clone());
    let locs_per_hour = use_state_eq(|| *state.locations_past_hour.borrow());

    // Subscribe to backend updates on new location data and number of locations
    // per hour
    let on_backend_msg = {
        let last_loc = last_loc.clone();
        let locs_per_hour = locs_per_hour.clone();
        move |msg: &ToFront| match msg {
            ToFront::LastLocation(val) => last_loc.set(Some(val.clone())),
            ToFront::LocationsPastHour(val) => locs_per_hour.set(Some(*val)),
            _ => (),
        }
    };
    // Generate a unique ID which doesn't change between renders since no deps
    // are given to use_memo
    let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    wss.subscribe(*id, Box::new(on_backend_msg));

    // send distance filter to backend
    let dist_filt = use_state_eq(|| *state.distance_filter.borrow());
    let onchange = {
        let dist_filt = dist_filt.clone();
        Callback::from(move |e: Event| {
            let input_elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if let Ok(val) = input_elem.value().parse::<f32>() {
                dist_filt.set(val);
                wss.send_msg(ToBack::SetDistFilt(val));
                swift_poke::poke(); // tell swift code to get new dist filt
            }
            input_elem.set_value("");
        })
    };
    html! {
        <>
            // deref then ref since we don't want to move the data out
            if let Some(loc) = &*last_loc {
                <p class="font-bold">
                    {"Current Location:"}
                </p>
                <p class="mb-4">
                    {format!("{:.5}, {:.5}", loc.lat, loc.lon)}
                </p>

                if let Some(n_locs) = *locs_per_hour {
                    <p class="whitespace-nowrap">
                        {format!("Num data points in past hour: {}", n_locs)}
                    </p>
                    <p class="mb-4">
                        {format!("{:.2} updates/minute", n_locs as f32 / 60.0)}
                    </p>
                }

                <div class="flex justify-center mb-4">
                    <p class="mr-6">
                        {"Distance Filter:"}
                    </p>
                    <input id="dist_filt"
                            placeholder={format!("{:.2}", *dist_filt)}
                            onchange={onchange}
                            class="w-16 rounded bg-neutral-900 \
                            border border-neutral-700 \
                            placeholder:text-neutral-200" />
                    <p class="ml-2">
                        {"m"}
                    </p>
                </div>

                <button class="rounded-lg whitespace-nowrap \
                        py-1.5 px-3 text-sky-500 bg-neutral-800">
                    {"Share SQLite log"}
                </button>

            } else {
                <p class="mb-4">
                    {"No recent location data found."}
                </p>
            }
        </>
    }
}
