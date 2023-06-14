//! Simple empty page to show while the UI state is loaded from the backend.
//! Reduces flickering.

use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::navigate_to_last_page;
use crate::websocket::{use_backend_event, ToFront};

/// Splash page when the app is loading. Redirects to the sense page when the
/// last bit of app state is received (ToFront::LocationsPastHour)
#[function_component]
pub fn Splash() -> Html {
    let navigator = use_navigator().unwrap();
    let on_get_state = {
        move |msg: &ToFront| {
            if let ToFront::FrontState(s) = msg {
                navigate_to_last_page(s, &navigator);
            }
        }
    };
    use_backend_event(on_get_state);
    html! {
        <>
        </>
    }
}
