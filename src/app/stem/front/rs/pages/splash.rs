//! Simple empty page to show while the UI state is loaded from the backend.
//! Reduces flickering.

use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::router::navigate_to_last_page;
use crate::ui_state::{init_state, FrontState};
use crate::websocket::{use_backend_event, ToFront};

/// Splash page when the app is loading.
///
/// Initializes the frontend state based on what's received from the backend,
/// then redirects to the past saved route.
#[function_component]
pub fn Splash() -> Html {
    crate::debug_with_time("splash");
    let navigator = use_navigator().unwrap();
    let front_dispatch = Dispatch::<FrontState>::new();
    let on_get_state = {
        move |msg: &ToFront| {
            if let ToFront::Startup(state, events) = msg {
                crate::debug_with_time("got frontstate");
                init_state(state, events);
                navigate_to_last_page(&front_dispatch.get(), &navigator);
            }
        }
    };
    use_backend_event(on_get_state);
    html! {}
}
