//! Component that shows the contents of a pin and allows editing of it.

use std::str::FromStr;

use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use common::{state::MapSettingsTab, LngLat, ToBack, ToFront};

use crate::{
    ui_state::{DerivedState, FrontState},
    websocket::{use_backend_event, WebsocketService},
};

static EDITABLE_INPUT_STYLE: &str =
    "rounded bg-neutral-800 border border-neutral-700";
static NONEDITABLE_INPUT_STYLE: &str = "rounded bg-neutral-900";

// Maximum number of characters (code points) for various fields
const ICON_MAX_CHARS: usize = 8;
const NAME_MAX_CHARS: usize = 128;
const LIST_MAX_CHARS: usize = 128;
const TAG_KEY_MAX_CHARS: usize = 128;
const TAG_VAL_MAX_CHARS: usize = 1024;

// future TODO:
// - ability to drag pin when editing (for now can just copy over the coords
// from a point at the desired location)
// - multiline display of pin fields

#[function_component]
pub fn PinEditor() -> Html {
    // pin to show in the editor
    let pin = use_selector(|s: &FrontState| s.map.current_pin.clone());
    // pin id last selected on the map (may be different)
    let selected_pin_id = use_selector(|s: &FrontState| s.map.selected_pin_id);
    let front_dispatch = Dispatch::<FrontState>::new();

    // whether edit mode is on or not
    let editing = use_selector(|s: &FrontState| s.map.editable_pin);

    let find_selected_pin = {
        let selected_pin_id = selected_pin_id.clone();
        Callback::from(move |_: ()| {
            let back_pins = &Dispatch::<DerivedState>::new().get().pins;
            return back_pins
                .iter()
                .find(|p| selected_pin_id.is_some() && *selected_pin_id == p.id)
                .cloned();
        })
    };

    if pin.id != *selected_pin_id && !*editing {
        // if we're not currently editing and the map selected id is different,
        // find the matching pin from the backend pin list to set to the pin
        if let Some(matching_pin) = find_selected_pin.emit(()) {
            front_dispatch.reduce_mut(move |state: &mut FrontState| {
                state.map.current_pin = matching_pin;
            });
        }
    }

    // show the input for entering a new list item
    let show_list_input = use_state(|| false);
    let show_list_input_onclick = {
        let show_list_input = show_list_input.clone();
        Callback::from(move |_| show_list_input.set(true))
    };

    // Editability
    let marked_for_deletion = use_state(|| false);
    let edit_cancel_onclick = {
        let marked_for_deletion = marked_for_deletion.clone();
        let show_list_input = show_list_input.clone();
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            if state.map.editable_pin {
                // just hit cancel -> close editor if it was a new pin
                if state.map.current_pin.id.is_none() {
                    state.map.settings_tab = MapSettingsTab::None;
                } else if let Some(matching_pin) = find_selected_pin.emit(()) {
                    // or revert the current pin data
                    state.map.current_pin = matching_pin;
                }
            }
            marked_for_deletion.set(false);
            state.map.editable_pin = !state.map.editable_pin;
            show_list_input.set(false);
        })
    };
    // Save button saves changes or executes the deletion if th delete button is
    // slelected.
    let save_onclick = {
        let marked_for_deletion = marked_for_deletion.clone();
        let wss = use_context::<WebsocketService>().unwrap();
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            if *marked_for_deletion {
                if let Some(id) = state.map.current_pin.id {
                    // has id, so was persisted to backend, delete it
                    wss.send_msg(ToBack::DeletePin(id));
                    // clear the current pin
                    state.map.current_pin = Default::default();
                }
                state.map.settings_tab = MapSettingsTab::None;
            } else {
                wss.send_msg(ToBack::SavePin(state.map.current_pin.clone()));
            }
            state.map.editable_pin = !state.map.editable_pin;
        })
    };
    let delete_onclick = {
        let marked_for_deletion = marked_for_deletion.clone();
        Callback::from(move |_| {
            marked_for_deletion.set(!*marked_for_deletion);
        })
    };
    let (delete_button_style, save_button_style) = if *marked_for_deletion {
        ("bg-red-500 text-white", "text-red-500")
    } else {
        ("bg-neutral-800 text-red-500", "text-primary")
    };
    let input_style = if *editing {
        EDITABLE_INPUT_STYLE
    } else {
        NONEDITABLE_INPUT_STYLE
    };

    let icon_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let mut s = elem.value();
            // Truncate anything longer than 8 unicode code points. This
            // allows for some grapheme clusters but nothing crazy.
            trunc_to_char(&mut s, ICON_MAX_CHARS);
            if s.trim().is_empty() {
                // new value is all whitespace -> don't allow change since this
                // will confusingly result in an invisible icon on the map
                elem.set_value(&state.map.current_pin.icon);
                return;
            }
            state.map.current_pin.icon = s;
        },
    );
    let name_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let mut s = elem.value();
            trunc_to_char(&mut s, NAME_MAX_CHARS);
            state.map.current_pin.name = s;
        },
    );

    let location_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let Ok(newlnglat) = LngLat::from_str(&elem.value()) else {
                // Set the elemet value explicitly, since there is no
                // re-render when we don't set new state.
                elem.set_value(&state.map.current_pin.lnglat.to_string());
                return;
            };
            if !newlnglat.is_valid() {
                // lnglat not on the globe, revert
                elem.set_value(&state.map.current_pin.lnglat.to_string());
                return;
            }
            state.map.current_pin.lnglat = newlnglat;
        },
    );

    // Lists
    let remove_list_item = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, idx: usize| {
            state.map.current_pin.lists.remove(idx);
        },
    );
    let list_elems = pin.lists.iter().enumerate().map(|(i, l)| {
        let editing = editing.clone();
        let remove_list_item = remove_list_item.clone();
        let remove_onclick = Callback::from(move |_| remove_list_item.emit(i));
        html! {
            <div class="flex items-center bg-neutral-800 px-2 py-1 \
                 rounded text-sm gap-1"
            >
                <span>{l.clone()}</span>
                if *editing {
                    <button onclick={remove_onclick}>
                        <Icon icon_id={IconId::BootstrapX}
                            class="h-5 w-5 text-neutral-400" />
                    </button>
                }
            </div>
        }
    });
    let list_input_onchange = {
        let show_list_input = show_list_input.clone();
        front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlInputElement = e.target_dyn_into().unwrap();
                let mut s = elem.value();
                trunc_to_char(&mut s, LIST_MAX_CHARS);
                state.map.current_pin.lists.push(s);
                show_list_input.set(false);
            },
        )
    };

    // Tags
    let tags_elems = pin.tags.iter().enumerate().map(|(i, t)| {
        let editing = editing.clone();
        let remove_onclick = {
            front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
                state.map.current_pin.tags.remove(i);
            })
        };
        let update_tag_onchange = {
            let front_dispatch = front_dispatch.clone();
            move |update_tag_key| {
                front_dispatch.reduce_mut_callback_with(
                    move |state: &mut FrontState, e: Event| {
                        let elem: HtmlInputElement =
                            e.target_dyn_into().unwrap();
                        let mut s = elem.value();
                        if update_tag_key {
                            trunc_to_char(&mut s, TAG_KEY_MAX_CHARS);
                            state.map.current_pin.tags[i].0 = s;
                        } else {
                            trunc_to_char(&mut s, TAG_VAL_MAX_CHARS);
                            state.map.current_pin.tags[i].1 = s;
                        }
                    },
                )
            }
        };
        html! {
            <div class="flex items-center justify-between flex-wrap gap-2">
                <input
                    value={t.0.clone()}
                    class={format!("w-24 mr-2 {}", input_style)}
                    onchange={update_tag_onchange(true)}
                    readonly={!*editing}
                />
                <input
                    value={t.1.clone()}
                    class={format!("w-32 flex-grow {}", input_style)}
                    onchange={update_tag_onchange(false)}
                    readonly={!*editing}
                />
                if *editing {
                    <button onclick={remove_onclick}
                        class="p-1 bg-neutral-800 rounded"
                    >
                        <Icon icon_id={IconId::BootstrapX}
                            class="h-5 w-5 text-neutral-400" />
                    </button>
                }
            </div>
        }
    });
    let add_tag_onclick =
        front_dispatch.reduce_mut_callback(|state: &mut FrontState| {
            state.map.current_pin.tags.push(("".into(), "".into()));
        });

    let close_settings_tab =
        front_dispatch.reduce_mut_callback(|state: &mut FrontState| {
            state.map.settings_tab = MapSettingsTab::None
        });

    let open_in_google_maps =
        use_selector(|state: &FrontState| state.map.open_in_google_maps);
    let maps_link = if cfg!(feature = "android_config") || *open_in_google_maps
    {
        format!("https://maps.google.com/?q={}", pin.lnglat)
    } else {
        format!("https://maps.apple.com/?q={}", pin.lnglat)
    };

    // update pin id for a newly saved pin, that's been assigned an id by the
    // backend
    use_backend_event(|msg: &ToFront| {
        if let ToFront::NewPinId(id) = msg {
            Dispatch::<FrontState>::new().reduce_mut(
                |state: &mut FrontState| {
                    state.map.current_pin.id = Some(*id);
                },
            );
        }
    });

    // Resize the map on any state change. Easier than tracking what things
    // cause the size of the PinEditor to change.
    use_effect(move || {
        let event = web_sys::Event::new("resize").unwrap();
        web_sys::window().unwrap().dispatch_event(&event).unwrap();
    });
    html! {
        <div class="flex m-1">
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg \
            overflow-hidden px-4 py-2 flex flex-col gap-2 max-w-prose"
        >
            <div class="flex items-center justify-start flex-wrap gap-1">
                <input
                    id="icon"
                    class={format!("w-8 text-lg {}", input_style)}
                    value={pin.icon.clone()}
                    onchange={icon_onchange}
                    readonly={!*editing}
                />
                <input
                    id="name"
                    class={format!("text-wrap text-lg font-medium grow {}",
                        input_style)}
                    value={pin.name.clone()}
                    onchange={name_onchange}
                    readonly={!*editing}
                />
            </div>
            <div class="flex items-center justify-between flex-wrap gap-4">
                <label for="latlng">{"Location"}</label>
                <input
                    id="latlng"
                    value={pin.lnglat.to_string()}
                    class={format!("w-48 flex-grow text-sm {}",
                        input_style)}
                    onchange={location_onchange}
                    readonly={!*editing}
                />
            </div>
            <div class="flex items-center justify-start flex-wrap gap-2">
                <label for="lists" class="mr-2">{"Lists"}</label>
                { for list_elems }
                if *editing && *show_list_input {
                    <input
                        id="lists"
                        value={""}
                        onchange={list_input_onchange}
                        class={format!("w-24 {}", input_style)}
                    />
                } else if *editing {
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
            if *editing {
                <div class="flex justify-start">
                    <button class="p-1 bg-neutral-800 rounded"
                        onclick={add_tag_onclick}
                    >
                        <Icon icon_id={IconId::BootstrapPlus}
                            class="h-6 w-6 text-neutral-400" />
                    </button>
                </div>
            }
            <div class="flex items-center justify-between flex-wrap gap-2">
                if *editing {
                    <button class={format!("px-3 py-1.5 flex items-center \
                        rounded-lg {}", delete_button_style)}
                        onclick={delete_onclick}
                    >
                        <Icon icon_id={IconId::BootstrapTrash}
                            class="mr-2 h-5 w-5" />
                        {"Delete"}
                    </button>
                    <button class="px-3 py-1.5 text-primary flex items-center \
                        rounded-lg bg-neutral-800"
                        onclick={edit_cancel_onclick}
                    >
                        {"Cancel"}
                    </button>
                    <div></div>
                    <button class={format!("px-3 py-1.5 rounded-lg \
                        bg-neutral-800 {}", save_button_style)}
                        onclick={save_onclick}
                    >
                        {"Save"}
                    </button>
                } else {
                    <a
                        class="py-1 text-primary flex gap-2 items-center"
                        href={maps_link}
                    >
                        {"Open in Maps"}
                        <Icon icon_id={IconId::BootstrapBoxArrowUpRight}
                            class="h-5 w-5" />
                    </a>
                    <div></div>
                    <button class="px-3 py-1.5 text-primary rounded-lg \
                        bg-neutral-800"
                        onclick={edit_cancel_onclick}
                    >
                        {"Edit"}
                    </button>
                    <button class="px-3 py-1.5 text-primary rounded-lg \
                        bg-neutral-800"
                        onclick={close_settings_tab}
                    >
                        {"Close"}
                    </button>
                }
            </div>
        </div>
        </div>
    }
}

/// Truncate to `n` unicode code points.
///
/// A normal string truncation may panic if the byte length splits a code point,
/// so this avoid the pitfall.
fn trunc_to_char(s: &mut String, n: usize) {
    let upto = s.char_indices().map(|(i, _)| i).nth(n).unwrap_or(s.len());
    s.truncate(upto);
}
