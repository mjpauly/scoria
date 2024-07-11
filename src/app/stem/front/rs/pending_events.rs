//! Hook for handling pending events, delivered just after the UI initializes.
//!
//! E.g. going to a clicked-on scoria:// link.

use common::pin::Pin;
use common::state::MapSettingsTab;
use common::ToFront;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::{
    router::Route, ui_state::FrontState, websocket::use_backend_event,
};

#[function_component]
pub fn PendingEventHandler() -> Html {
    use_pending_events();
    html! {}
}

/// Handle pending events, such as opening a scoria:// link, which is delivered
/// before the UI is active.
#[hook]
pub fn use_pending_events() {
    let navigator = use_navigator().unwrap();
    let front_dispatch = Dispatch::<FrontState>::new();
    let on_open_url = move |msg: &ToFront| {
        let ToFront::PendingEvents(events) = msg else {
            return;
        };
        let Some(url) = &events.opened_url else {
            return;
        };
        if let Ok(pin) = Pin::from_url(url.clone()) {
            front_dispatch.reduce_mut(|s: &mut FrontState| {
                s.map.view_pos.center = pin.lnglat;
                s.map.view_pos.zoom = 16.0;
                s.map.current_pin = pin;
                s.map.editable_pin = true;
                s.map.settings_tab = MapSettingsTab::PinDetails;
            });
            navigator.push(&Route::Analyze);
        }
    };
    use_backend_event(on_open_url);
}
