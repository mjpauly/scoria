//! Configuration of sensors, like GPS, audio, etc.

use yew::prelude::*;

use crate::NavbarWrapper;

#[function_component]
pub fn Sense() -> Html {
    html! {
        <>
            <LocationConfig />
        </>
    }
}

#[function_component]
fn LocationConfig() -> Html {
    html! {
        <NavbarWrapper>
            <h1 class="text-sky-500 text-3xl mb-6">
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
                <p class="text-sky-500 ml-2">
                    {"?"}
                </p>
            </div>

            <button class="self-center rounded-lg w-min whitespace-nowrap \
                    py-1.5 px-3 text-sky-500 bg-neutral-800">
                {"Share SQLite log"}
            </button>
        </NavbarWrapper>
    }
}
