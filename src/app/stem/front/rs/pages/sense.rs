//! Configuration of sensors, like GPS, audio, etc.

// Profile how long things take with:
// log::debug!(
// "refreshing at {:?}",
// web_sys::window().unwrap().performance().unwrap().now() as i64 % 1000
// );

use yew::prelude::*;

use crate::components::{
    map_styler::use_check_epsln_tile_server, LocationConfigurator,
    NavbarWrapper,
};
use crate::ui_state::UIState;
use crate::websocket::use_backend_event;
use crate::websocket::ToFront;

#[function_component]
pub fn Sense() -> Html {
    use_check_epsln_tile_server();
    html! {
        <>
            <Location />
        </>
    }
}

#[function_component]
fn Location() -> Html {
    html! {
        <NavbarWrapper>
            <div class="flex flex-col h-full pt-16">
                // my-auto: center vertically in the flex (better than
                //      justify-center, which ALWAYS centers and thus cuts off
                //      content if it's too big for the container)
                <div class="my-auto">
                    <h1 class="text-primary text-3xl mb-6">
                        {"Location"}
                    </h1>

                    <LocationDetails />
                    <p class="mt-6 font-bold"> {"Settings"} </p>
                    <LocationConfigurator />

                </div>
            </div>
        </NavbarWrapper>
    }
}

/// Details about the last location received and the amount of data recorded
/// recently.
#[function_component]
fn LocationDetails() -> Html {
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
    use_backend_event(on_backend_msg);

    // Update the current state of `now` every second, so things update even if
    // there's no new data coming from the backend
    let now = use_state(|| time::OffsetDateTime::now_local().unwrap());
    {
        let now = now.clone();
        yew_hooks::use_interval(
            move || {
                now.set(time::OffsetDateTime::now_local().unwrap());
            },
            1_000,
        );
    }
    // Time since last location data in human readable form
    let mut time_since = String::from("");
    if let Some(loc) = &*last_loc {
        let dur = *now - loc.datetime;
        if dur > time::Duration::seconds(2) {
            time_since = format!(
                "{} ago",
                humantime::format_duration(
                    time::Duration::seconds(dur.whole_seconds()).unsigned_abs()
                )
            );
        } else {
            time_since = String::from("<2s ago");
        }
    }
    html! {
        <>
            // deref then ref since we don't want to move the data out
            if let Some(loc) = &*last_loc {
                <p class="font-bold mb-2"> {"Last Location"} </p>
                <p> {format!("{:.5}, {:.5}", loc.lat, loc.lon)} </p>
                <p> {format!("+/-{:.2} m, {:.2} m/s, {:.2}°",
                             loc.accuracy, loc.speed, loc.course)}
                </p>
                <p class="mb-4"> {time_since} </p>
                if let Some(n_locs) = *locs_per_hour {
                    <p class="whitespace-nowrap">
                        {format!("Num data points in past hour: {}", n_locs)}
                    </p>
                    <p class="mb-4">
                        {format!("{:.2} updates/minute", n_locs as f32 / 60.0)}
                    </p>
                }

            } else {
                <p class="mb-4"> {"No previous location data found."} </p>
            }
        </>
    }
}
