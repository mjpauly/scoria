//! Hook for handling pending events that may be delivered while the app is
//! open.
//!
//! Events that arrive before the app is opened are handled by
//! ui_state::init_state(), e.g. when clicking on a scoria:// link in a
//! different app

use common::ToFront;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{ui_state::handle_pending_events, websocket::use_backend_event};

#[function_component]
pub fn PendingEventHandler() -> Html {
    use_pending_events();
    html! {}
}

/// Handle pending events, such as opening a scoria:// link.
#[hook]
pub fn use_pending_events() {
    let navigator = use_navigator().unwrap();
    let on_open_url = move |msg: &ToFront| {
        let ToFront::PendingEvents(events) = msg else {
            return;
        };
        if let Some(route_to_go_to) =
            handle_pending_events(&Some(events.clone()))
        {
            navigator.push(&route_to_go_to);
        }
    };
    use_backend_event(on_open_url);
}
