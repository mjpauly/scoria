//! Navigation components for switching between pages. px are used where buffer
//! is created for the device's home bar and top notch.

use yew::prelude::*;
use yew::MouseEvent;
use yew_router::prelude::*;

use crate::router::Route;

/// Button to place in a TopNav (on the right) which will go to the Sense page.
#[function_component]
pub fn DoneButton() -> Html {
    let navigator = use_navigator().unwrap();
    let exit_settings_onclick =
        Callback::from(move |_e: MouseEvent| navigator.push(&Route::Sense));
    html! {
        <button class="text-primary p-2 px-6"
            onclick={exit_settings_onclick}>
            <label id="done">{"Done"}</label>
        </button>
    }
}
