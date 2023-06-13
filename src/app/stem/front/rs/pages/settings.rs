//! Deeper app settings, such as data log exporting

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::components::unit_picker::UnitPicker;
use crate::router::Route;
use crate::swift_poke;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn Settings() -> Html {
    let navigator = use_navigator().unwrap();
    let exit_settings_onclick = Callback::from(move |_e: MouseEvent| {
        navigator.push(&Route::Sense {
            scope: Route::get_scope(),
        })
    });
    html! {
        <div class="flex flex-col h-screen">
            <button onclick={exit_settings_onclick}
                class="absolute right-6 top-14 p-2 \
                rounded-lg bg-neutral-800 text-neutral-200">
                <Icon icon_id={IconId::BootstrapX} class="h-6 w-6" />
            </button>
            <div class="my-auto">
                <DataLogSettings />
            </div>
        </div>
    }
}

#[function_component]
pub fn DataLogSettings() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let export_onclick = {
        let wss = wss.clone();
        Callback::from(move |_e: MouseEvent| {
            wss.send_msg(ToBack::ExportSqliteLog);
            swift_poke::poke();
        })
    };
    let import_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ImportSqliteLog);
        swift_poke::poke();
    });

    let navigator = use_navigator().unwrap();
    let show_intro_onclick = Callback::from(move |_e: MouseEvent| {
        navigator.push(&Route::Intro {
            scope: Route::get_scope(),
        })
    });
    html! {
        // flex container for centering
        <div class="mt-2 mb-1 flex px-4">
        // centered, width-limited container
        <div class="grow max-w-prose mx-auto">

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2 mb-5">
            // settings line
            <button onclick={show_intro_onclick}
                class="flex items-center justify-between py-2 w-full">
                <span class="text-primary">
                    {"Show Epsilon Introduction"}
                </span>
            </button>
        </div>

        <UnitPicker/>

        // title
        <div class="relative">
            <p class="font-bold"> {"Data"} </p>
        </div>

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
            // settings line
            <button onclick={export_onclick}
                class="flex items-center justify-between py-2 w-full">
                <span class="text-primary">
                    {"Export Log"}
                </span>
            </button>
        </div>
        <p class="text-neutral-500 text-left px-2 pt-1">
            {"Exported logs can be imported back into Epsilon. You can
            use this to backup your data or migrate between devices."}
        </p>

        // settings card
        <div class="bg-neutral-900 rounded-lg px-4 py-1 mt-2">
            // settings line
            <button onclick={import_onclick}
                class="flex items-center justify-between py-2 w-full">
                <span class="text-primary">
                    {"Import Log"}
                </span>
            </button>
        </div>
        <p class="text-neutral-500 text-left px-2 pt-1">
            {"Import an Epsilon log that was previously exported. The data will
            be added to your current log. Duplicate data points are determined
            by timestamp, and are not imported."}
        </p>
        <p class="text-neutral-500 text-left px-2 pt-1">
            {"Since imported data is irreversibly added to your log, using this
            feature for looking at data that is not your own is not recommended.
            Let us know if you want this kind of feature."}
        </p>

        </div>
        </div>
    }
}
