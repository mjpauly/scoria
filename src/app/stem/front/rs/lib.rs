//! Frontend library containing all components. A small main.rs file starts up
//! the app.
//!
//! # Dev Notes
//!
//! TailwindCSS classes must be spelled out in Rust files for them to be picked
//! up. Don't do this:
//!
//! ```
//! let scale = 8;
//! let style: String = format!("rouned h-{} w-{}", scale, scale);
//! ```
//!
//! Do this instead:
//!
//! ```
//! let scale = "h-8 w-8"; // tailwind can find these classes and include them
//! let style: String = format!("rouned {}", scale);
//! ```

mod components;
mod logs;
mod maplibre;
mod pages;
mod pending_events;
mod plotly;
mod router;
mod swift_poke;
mod ui_state;
mod unwrapping;
mod web;
mod websocket;

use yew::prelude::*;
use yew_router::prelude::*;

use websocket::WebsocketService;

use crate::components::db_loading_banner::DbLoadingBanner;
use crate::components::toast::Toast;

/// Top level App component for the UI.
#[function_component]
pub fn App() -> Html {
    // Start logging and get a handle to reload with the websocket log listener
    let handle = logs::init_logging();

    // Start the websocket service, to be made available to all components with
    // `use_context`
    let wss = WebsocketService::new();

    // Reload the logging
    logs::reload_with_ws_log_listener(handle, wss.get_sender());

    // Create an id for this function component to associate our callback with,
    // and register the app state update callback.
    let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    wss.subscribe(*id, ui_state::get_update_callback());

    // Spawn a future that regularly requests updated state info from backend
    wss.clone().spawn_state_requester();

    // Notify backend whenever the UI state changes
    ui_state::init_front_listener(wss.clone());

    // In debug builds, fall back to simulated safe-area insets when the
    // environment provides none, for screenshotting in Firefox's device
    // simulation
    #[cfg(debug_assertions)]
    web::sim_insets::apply_fallback();

    // Get the frontend key / scope to use as the router basename
    let basename = format!("/{}", router::get_scope());

    html! {
        // default parent style for the UI which pages inherit
        <div class="text-center bg-black flex-1 flex flex-col \
                    font-light text-neutral-200 select-none">
            <ContextProvider<WebsocketService> context={wss}>
                <Toast />
                <DbLoadingBanner />
                <BrowserRouter basename={basename}>
                    <Switch<router::Route> render={router::switch} />
                    <pending_events::PendingEventHandler />
                </BrowserRouter>
            </ContextProvider<WebsocketService>>
        </div>
    }
}

pub fn now() -> i64 {
    web_sys::window().unwrap().performance().unwrap().now() as i64 % 1000
}

pub fn debug_with_time(s: &str) {
    tracing::debug!("{} ms, {}", now(), s);
}
