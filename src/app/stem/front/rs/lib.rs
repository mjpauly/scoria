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
mod event_bus;
mod pages;
mod router;
mod websocket;

use yew::prelude::*;
use yew_router::prelude::*;

/// Top level App component for the UI.
#[function_component]
pub fn App() -> Html {
    html! {
        // default parent style for the UI which pages inherit
        <div class="place-content-center text-center flex flex-col \
                    min-h-screen bg-neutral-900 \
                    font-light text-neutral-200 select-none">
                    // background gradients should work behind navbar!
                    // bg-gradient-to-b from-purple-900 to-pink-900">
            <BrowserRouter>
                <Switch<router::Route> render={router::switch} />
            </BrowserRouter>
        </div>
    }
}
