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

mod common;
mod components;
mod pages;
mod router;
mod ui_state;
mod websocket;

use yew::prelude::*;
use yew_router::prelude::*;

use ui_state::UIState;
use websocket::WebsocketService;

/// Top level App component for the UI.
#[function_component]
pub fn App() -> Html {
    // Start our websocket service, so we use it as a context available to all
    // components with `use_context`
    let wss = WebsocketService::new();
    let state = UIState::new();
    let id = use_memo(|_| WebsocketService::gen_callback_id(), ());
    wss.subscribe(*id, state.get_update_callback());
    // TODO: ask backend for app state at start

    html! {
        // default parent style for the UI which pages inherit
        <div class="place-content-center text-center flex flex-col \
                    min-h-screen bg-neutral-900 \
                    font-light text-neutral-200 select-none">
                    // background gradients should work behind navbar!
                    // bg-gradient-to-b from-purple-900 to-pink-900">
            <ContextProvider<WebsocketService> context={wss}>
            <ContextProvider<UIState> context={state}>
                <BrowserRouter>
                    <Switch<router::Route> render={router::switch} />
                </BrowserRouter>
            </ContextProvider<UIState>>
            </ContextProvider<WebsocketService>>
        </div>
    }
}
