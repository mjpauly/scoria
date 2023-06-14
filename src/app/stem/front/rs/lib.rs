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
//! let scale = "h-8 w-8"; // tailwind can find these classes and include them
//! let style: String = format!("rouned {}", scale);

mod components;
mod pages;
mod plots;
mod router;
mod swift_poke;
mod ui_state;
mod websocket;

use yew::prelude::*;
use yew_router::prelude::*;

use websocket::WebsocketService;

/// Top level App component for the UI.
#[function_component]
pub fn App() -> Html {
    // Start our websocket service, so we use it as a context available to all
    // components with `use_context`
    let wss = WebsocketService::new();
    // Create an id for this function component to associate our callback with
    let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    // Register the app state update callback first ahead of all children
    wss.subscribe(*id, ui_state::get_update_callback());
    // Spawn a future that regularly requests updated state info from backend
    wss.clone().spawn_state_requester();
    // Notify backend whenever the UI state changes
    ui_state::init_backend_listener(wss.clone());

    // Get the frontend key / scope to use as the router basename
    let basename = format!("/{}", router::get_scope());

    html! {
        // default parent style for the UI which pages inherit
        <div class="text-center bg-black \
                    font-light text-neutral-200 select-none">
                    // background gradients should work behind navbar!
                    // bg-gradient-to-b from-purple-900 to-pink-900">
            <ContextProvider<WebsocketService> context={wss}>
                <BrowserRouter basename={basename}>
                    <Switch<router::Route> render={router::switch} />
                </BrowserRouter>
            </ContextProvider<WebsocketService>>
        </div>
    }
}
