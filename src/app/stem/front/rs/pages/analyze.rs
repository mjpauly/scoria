//! Analysis of collected data.

use yew::prelude::*;

use crate::components::NavbarWrapper;

#[function_component]
pub fn Analyze() -> Html {
    html! {
        <NavbarWrapper>
            <h1 class="text-sky-500 text-3xl mb-6">
                {"Analyze!"}
            </h1>
        </NavbarWrapper>
    }
}
