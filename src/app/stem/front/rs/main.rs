// use stylist::{css, style, yew::styled_component};
use yew::prelude::*;

#[function_component]
fn LocationConfig() -> Html {
    html! {
        <>
            <h1 class="text-blu text-3xl mb-6">
                {"Location"}
            </h1>

            <p class="font-bold">
                {"Current Location:"}
            </p>
            <p class="mb-4">
                {"xxx, yyy"}
            </p>

            <p class="whitespace-nowrap">
                {"Num data points this hour: xxx"}
            </p>
            <p class="mb-4">
                {"xxx updates/minute"}
            </p>

            <div class="flex justify-center mb-4">
                <p class="mr-6">
                    {"Distance Filter"}
                </p>
                <input id="dist_filt" class="w-12 rounded bg-neutral-900 \
                        border border-neutral-700" />
                <p class="text-blu ml-2">
                    {"?"}
                </p>
            </div>

            <button class="self-center rounded-lg w-min whitespace-nowrap \
                    py-1.5 px-3 text-blu bg-neutral-800">
                {"Share SQLite log"}
            </button>
        </>
    }
}

#[function_component]
fn App() -> Html {
    html! {
        // parent style for the UI
        <div class="place-content-center flex flex-col h-screen \
                    bg-neutral-900 \
                    font-light text-neutral-100 text-center">
            <LocationConfig />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
