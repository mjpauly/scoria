//! Page for importing places from a GeoJSON file.
//!
//! Supports the data as a feature collection of point features.

use std::str::FromStr;

use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use common::LngLat;

use crate::components::buttons::{DoneButton, MainSettingsButton};
use crate::components::pin_editor::INPUT_STYLE;
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, HomeBarSpacer, SettingsCard,
    SettingsCardSimpleButton, TopNav, H1,
};
use crate::swift_poke;
use crate::ui_state::FrontState;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn ImportPlaces() -> Html {
    let wss = use_context::<WebsocketService>().unwrap();
    let import_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ImportPlaces);
        swift_poke::poke();
    });
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Import Places"} </H1>

                <p class="mt-2 mx-2 text-left">
                    {"Import places from a GeoJSON file. The file should contain
                    a FeatureCollection of Features with Point geometries.
                    Feature properties are saved as tags."}
                </p>

                <p class="mt-4 mb-4 mx-2 text-left">
                    {"Edit the default values for imported places below. The
                    icon, lists, and tags are copied over. The default name and
                    location are used only if they are not in the input data."}
                </p>

                <DefaultPinEditor />

                <AfterCardParagraph>
                    {"If the location is not found, the icon is also set to
                    ❓."}
                </AfterCardParagraph>

                <SettingsCard class="mt-4">
                    <SettingsCardSimpleButton
                        text="Import GeoJSON"
                        onclick={import_onclick}
                    />
                </SettingsCard>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>

    }
}

/// Show the default pin data used for annotating improted places.
///
/// Moslty copied over from PinEditor in pin_editor.rs
#[function_component]
fn DefaultPinEditor() -> Html {
    let pin = use_selector(|s: &FrontState| s.pin_import_default.clone());
    let front_dispatch = Dispatch::<FrontState>::new();

    // show the input for entering a new list item
    let show_list_input = use_state(|| false);
    let show_list_input_onclick = {
        let show_list_input = show_list_input.clone();
        Callback::from(move |_| show_list_input.set(true))
    };

    let icon_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if state.pin_import_default.set_icon(elem.value()).is_err() {
                // invalid, reset to previous value
                elem.set_value(&state.pin_import_default.icon);
            }
        },
    );
    let name_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
            state.pin_import_default.set_name(elem.value());
        },
    );

    let location_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let Ok(newlnglat) = LngLat::from_str(&elem.value()) else {
                // Set the elemet value explicitly, since there is no
                // re-render when we don't set new state.
                elem.set_value(&state.pin_import_default.lnglat.to_string());
                return;
            };
            if state.pin_import_default.set_lnglat(newlnglat).is_err() {
                // invalid, revert
                elem.set_value(&state.pin_import_default.lnglat.to_string());
            }
        },
    );

    // Lists
    let remove_list_item = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, idx: usize| {
            state.pin_import_default.lists.remove(idx);
        },
    );
    let list_elems = pin.lists.iter().enumerate().map(|(i, l)| {
        let remove_list_item = remove_list_item.clone();
        let remove_onclick = Callback::from(move |_| remove_list_item.emit(i));
        html! {
            <div class="flex items-center bg-neutral-800 px-2 py-1 \
                 rounded text-sm gap-1"
            >
                <span>{l.clone()}</span>
                <button onclick={remove_onclick}>
                    <Icon icon_id={IconId::BootstrapX}
                        class="h-5 w-5 text-neutral-400" />
                </button>
            </div>
        }
    });
    let list_input_onchange = {
        let show_list_input = show_list_input.clone();
        front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlInputElement = e.target_dyn_into().unwrap();
                state.pin_import_default.push_list(elem.value());
                show_list_input.set(false);
            },
        )
    };

    // Tags
    let tags_elems = pin.tags.iter().enumerate().map(|(i, t)| {
        let remove_onclick = {
            front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
                state.pin_import_default.tags.remove(i);
            })
        };
        let update_key_onchange = front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlInputElement = e.target_dyn_into().unwrap();
                state.pin_import_default.update_key_at(i, elem.value());
            },
        );
        let update_value_onchange = front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
                state.pin_import_default.update_val_at(i, elem.value());
            },
        );
        html! {
            <div class="flex items-center justify-between flex-wrap gap-2">
                <input
                    value={t.0.clone()}
                    class={format!("w-1/3 {}", INPUT_STYLE)}
                    onchange={update_key_onchange}
                    rows=1
                />
                <textarea
                    value={t.1.clone()}
                    class={format!("w-24 flex-grow text-sm {}", INPUT_STYLE)}
                    onchange={update_value_onchange}
                />
                <button onclick={remove_onclick}
                    class="p-1 bg-neutral-800 rounded"
                >
                    <Icon icon_id={IconId::BootstrapX}
                        class="h-5 w-5 text-neutral-400" />
                </button>
            </div>
        }
    });
    let add_tag_onclick =
        front_dispatch.reduce_mut_callback(|state: &mut FrontState| {
            state.pin_import_default.tags.push(("".into(), "".into()));
        });

    html! {
        <div class="flex text-left max-h-[50lvh] mx-1 overflow-scroll">
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg h-full \
            px-4 py-2 flex flex-col gap-2 max-w-prose my-1"
        >
            <div class="flex items-center justify-start gap-2">
                <input
                    id="icon"
                    class={format!("w-24 text-lg {}", INPUT_STYLE)}
                    value={pin.icon.clone()}
                    onchange={icon_onchange}
                />
                <textarea
                    id="name"
                    class={format!("text-wrap text-lg font-medium w-10 grow \
                        placeholder:text-neutral-500 {}", INPUT_STYLE)}
                    value={pin.name.clone()}
                    placeholder="Name"
                    onchange={name_onchange}
                />
            </div>
            <div class="flex items-center justify-between flex-wrap gap-4">
                <label for="latlng">{"Location"}</label>
                <input
                    id="latlng"
                    value={pin.lnglat.to_string()}
                    class={format!("w-24 flex-grow text-sm {}",
                        INPUT_STYLE)}
                    onchange={location_onchange}
                />
            </div>
            <div class="flex items-center justify-start flex-wrap gap-2">
                <label for="lists" class="mr-2">{"Lists"}</label>
                { for list_elems }
                if *show_list_input {
                    <input
                        id="lists"
                        value={""}
                        onchange={list_input_onchange}
                        class={format!("w-24 {}", INPUT_STYLE)}
                    />
                } else {
                    <button class="bg-neutral-800 p-1 \
                         rounded text-sm"
                         onclick={show_list_input_onclick}
                    >
                        <Icon icon_id={IconId::BootstrapPlus}
                            class="h-5 w-5 text-neutral-400" />
                    </button>
                }
            </div>
            { for tags_elems }
            <div class="flex justify-start">
                <button class="p-1 bg-neutral-800 rounded"
                    onclick={add_tag_onclick}
                >
                    <Icon icon_id={IconId::BootstrapPlus}
                        class="h-6 w-6 text-neutral-400" />
                </button>
            </div>
        </div>
        </div>
    }
}
