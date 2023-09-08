//! Configuration of sensors, like GPS, audio, etc.

// Profile how long things take with:
// log::debug!(
// "refreshing at {:?}",
// web_sys::window().unwrap().performance().unwrap().now() as i64 % 1000
// );

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::components::{LocationConfigurator, TabBar, TopNav};
use crate::router::SettingsRoute;
use crate::ui_state::{BackState, FrontState};
use common::{LocationAccuracyMode, LocationMode};

#[function_component]
pub fn Sense() -> Html {
    let navigator = use_navigator().unwrap();
    let settings_onclick = Callback::from(move |_e: MouseEvent| {
        navigator.push(&SettingsRoute::Root)
    });
    html! {
        <>
            <TopNav>
                <button class="text-primary flex items-center p-2 px-6"
                    onclick={settings_onclick}>
                    <Icon icon_id={IconId::BootstrapGear}
                        class="h-5 w-5 mr-2" />
                    <label>{"Settings"}</label>
                </button>
            </TopNav>
            <Location />
            <TabBar />
        </>
    }
}

#[function_component]
fn Location() -> Html {
    html! {
        <div class="grow overflow-scroll h-0 w-screen \
            flex flex-col">
            // my-auto: center vertically in the flex (better than
            //      justify-center, which ALWAYS centers and thus cuts off
            //      content if it's too big for the container)
            <div class="my-auto">
                <h1 class="text-primary text-3xl mb-6">
                    {"Location"}
                </h1>

                <LocationDetails />
                <LocationConfigurator />

            </div>
        </div>
    }
}

/// Details about the last location received and the amount of data recorded
/// recently.
#[function_component]
fn LocationDetails() -> Html {
    let last_loc =
        use_selector(|state: &BackState| state.last_location.clone());
    let locs_per_hour =
        use_selector(|state: &BackState| state.locations_past_hour);
    let user_config = use_selector(|s: &FrontState| s.location_config.clone());
    let auto_config =
        use_selector(|s: &BackState| s.auto_location_config.clone());
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref.clone());

    // Update the current state of `now` every second, so things update even if
    // there's no new data coming from the backend
    let now = use_state(|| time::OffsetDateTime::now_local().unwrap());
    {
        let now = now.clone();
        yew_hooks::use_interval(
            move || {
                now.set(time::OffsetDateTime::now_local().unwrap());
            },
            1_000, // millis
        );
    }
    let prev_loc_html = if let Some(loc) = &*last_loc {
        let dur = *now - loc.timestamp;
        // Time since last location data in human readable form
        let time_since = if dur > time::Duration::seconds(2) {
            format!(
                "{} ago",
                humantime::format_duration(
                    time::Duration::seconds(dur.whole_seconds()).unsigned_abs()
                )
            )
        } else {
            String::from("<2s ago")
        };
        let latlon = format!(
            "{}, {}",
            unit_pref.format_angle(loc.latitude, Some(5)),
            unit_pref.format_angle(loc.longitude, Some(5))
        );
        let mut accuracy_speed_course = format!(
            "±{}",
            unit_pref.format_small_length(loc.horizontal_accuracy, Some(2)),
        );
        if let Some(speed) = loc.speed {
            accuracy_speed_course +=
                &format!(", {}", unit_pref.format_velocity(speed, Some(2)));
        }
        if let Some(course) = loc.course {
            accuracy_speed_course +=
                &format!(", {}", unit_pref.format_angle(course, Some(2)));
        }
        let alt = loc.msl_altitude.map(|alt| {
            format!("{} altitude", unit_pref.format_small_length(alt, Some(2)))
        });
        html! {
            // allow selection of the current location details
            <div class="select-text">

            <p class="font-bold mb-2"> {"Last Location"} </p>
            <p> {latlon} </p>
            <p> {accuracy_speed_course} </p>
            if let Some(a) = alt {
                <p> {a} </p>
            }
            <p class="mb-4"> {time_since} </p>
            if let Some(n_locs) = *locs_per_hour {
                <p>
                    {format!("Num data points in past hour: {}", n_locs)}
                </p>
                <p class="mb-4">
                    {format!("{:.2} updates/minute", n_locs as f32 / 60.0)}
                </p>
            }

            </div>
        }
    } else {
        html! { <p class="mb-4"> {"No previous location data found."} </p> }
    };
    html! {
        <>
            {prev_loc_html}
            if user_config.mode == LocationMode::Auto {
                <p class="mb-4"> {format!("Auto mode accuracy: {}",
                    if auto_config.standard_config.accuracy_mode
                        == LocationAccuracyMode::Best {
                        "High"
                    } else {
                        "Low"
                    })}
                </p>
            }
        </>
    }
}
