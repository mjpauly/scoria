//! Simple empty page to show while the UI state is loaded from the backend.
//! Reduces flickering.

use common::state::PersistedRoute;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::pages::intro::INTRO_VERSION;
use crate::router::Route;
use crate::websocket::{use_backend_event, ToFront};

/// Splash page when the app is loading. Redirects to the sense page when the
/// last bit of app state is received (ToFront::LocationsPastHour)
#[function_component]
pub fn Splash() -> Html {
    let navigator = use_navigator().unwrap();
    let on_get_state = {
        move |msg: &ToFront| {
            if let ToFront::FrontState(s) = msg {
                // get the previously persisted route, if it exists
                let next = if let Some(state) = s {
                    if state.last_viewed_intro_version < INTRO_VERSION {
                        Route::from_persisted_route(&PersistedRoute::Intro)
                    } else {
                        Route::from_persisted_route(&state.route)
                    }
                } else {
                    Route::from_persisted_route(&PersistedRoute::Intro)
                };
                navigator.push(&next);
            }
        }
    };
    use_backend_event(on_get_state);
    html! {
        <>
        </>
    }
}
