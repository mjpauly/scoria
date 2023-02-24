// use stylist::{css, style, yew::styled_component};
use yew::prelude::*;

#[function_component]
fn LocationConfig() -> Html {
    html! {
        <>
            <h1>{"Location"}</h1>
            <p class="bold">{"Current Location:"}</p>
            <p class="linebreak"> {"xxx, yyy"} </p>

            <p>{"Num data points this hour: xxx"}</p>
            <p class="linebreak">{"xxx updates/minute"}</p>

            <p class="linebreak">{"Distance Filter ____"}</p>

            <button>{"Share SQLite log"}</button>
        </>
    }
}

#[function_component]
fn App() -> Html {
    html! {
        <div class="center">
            <LocationConfig />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
