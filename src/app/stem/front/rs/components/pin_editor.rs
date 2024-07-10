//! Component that shows the contents of a pin and allows editing of it.

use std::str::FromStr;

use common::{pin::Pin, state::MapSettingsTab, LngLat, ToBack, ToFront};
use unicode_segmentation::UnicodeSegmentation;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::{
    ui_state::{DerivedState, FrontState},
    websocket::{use_backend_event, WebsocketService},
};

static INPUT_STYLE: &str = "rounded bg-neutral-800 border border-neutral-700";
// prevent long words from increasing width of the element
static FREEFORM_TEXT_STYLE: &str =
    "whitespace-pre-wrap break-words table table-fixed w-full";

// Maximum number of characters (code points) for various fields
const ICON_MAX_GRAPHEMES: usize = 5; // max number of graphemes
const ICON_MAX_CHARS: usize = 32; // max number of code points (some emoji have
                                  // up to 4 code points)
const NAME_MAX_CHARS: usize = 128;
const LIST_MAX_CHARS: usize = 128;
const TAG_KEY_MAX_CHARS: usize = 128;
const TAG_VAL_MAX_CHARS: usize = 1024;

// future TODO:
// - ability to drag pin when editing (for now can just copy over the coords
// from a point at the desired location)
// - multiline display of pin fields

/// Top-level pin details component
#[function_component]
pub fn PinDetails() -> Html {
    let editing = use_selector(|s: &FrontState| s.map.editable_pin);

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

    html! {
        if *editing {
            <PinEditor />
        } else {
            <PinViewer />
        }
    }
}

/// Find the pin from the derived state that has the matching id as the pin
/// selected on the map.
fn find_matching_pin(selected_pin_id: &Option<i64>) -> Option<Pin> {
    let back_pins = &Dispatch::<DerivedState>::new().get().pins;
    back_pins
        .iter()
        .find(|p| selected_pin_id.is_some() && *selected_pin_id == p.id)
        .cloned()
}

/// Views pin details without editing.
#[function_component]
pub fn PinViewer() -> Html {
    // pin to show in the editor
    let pin = use_selector(|s: &FrontState| s.map.current_pin.clone());
    // pin id last selected on the map (may be different)
    let selected_pin_id = use_selector(|s: &FrontState| s.map.selected_pin_id);
    let front_dispatch = Dispatch::<FrontState>::new();

    if pin.id != *selected_pin_id {
        // if the map selected id is different, find the matching pin from the
        // backend pin list to set to the pin
        if let Some(matching_pin) = find_matching_pin(&selected_pin_id) {
            front_dispatch.reduce_mut(move |state: &mut FrontState| {
                state.map.current_pin = matching_pin;
            });
        }
    }

    let edit_onclick =
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            state.map.editable_pin = true;
        });
    let list_elems = pin.lists.iter().map(|l| {
        html! {
            <span class="bg-neutral-800 px-2 py-1 rounded text-sm">
                {l.clone()}
            </span>
        }
    });

    // Tags
    let tags_elems = pin.tags.iter().map(|t| {
        html! {
            <div class="flex items-center justify-start gap-2 select-text">
                <span class="w-1/3"> {t.0.clone()} </span>
                <div class="w-2/3 max-h-32 overflow-scroll">
                    <span class={format!("text-sm {}", FREEFORM_TEXT_STYLE)}>
                        {t.1.clone()}
                    </span>
                </div>
            </div>
        }
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

    // Resize the map on any state change
    use_effect(move || {
        let event = web_sys::Event::new("resize").unwrap();
        web_sys::window().unwrap().dispatch_event(&event).unwrap();
    });
    html! {
        <div class="flex m-1 text-left">
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg \
            overflow-hidden px-4 py-2 flex flex-col gap-2 max-w-prose"
        >
            <div class="flex items-center justify-start gap-2 select-text \
                text-lg"
            >
                <span class="whitespace-nowrap"> {pin.icon.clone()} </span>
                <div class="max-h-32 overflow-scroll font-medium">
                    <span class={format!("grow {}", FREEFORM_TEXT_STYLE)}>
                        {pin.name.clone()}
                    </span>
                </div>
            </div>
            <div class="flex items-center justify-start gap-2 select-text">
                <label for="latlng" class="w-1/3">{"Location"}</label>
                <span id="latlng" class="w-2/3 text-sm" >
                    {pin.lnglat.to_string()}
                </span>
            </div>
            if list_elems.len() != 0 {
                <div class="flex items-center justify-start gap-2 select-text">
                    <label for="lists" class="w-1/3">{"Lists"}</label>
                    <div id="lists" class="w-2/3 flex flex-wrap justify-start \
                        gap-2"
                    >
                        { for list_elems }
                    </div>
                </div>
            }
            { for tags_elems }
            <div class="flex items-center justify-between flex-wrap gap-2">
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
                    onclick={edit_onclick}
                >
                    {"Edit"}
                </button>
                <button class="px-3 py-1.5 text-primary rounded-lg \
                    bg-neutral-800"
                    onclick={close_settings_tab}
                >
                    {"Close"}
                </button>
            </div>
        </div>
        </div>
    }
}

#[function_component]
pub fn PinEditor() -> Html {
    // pin to show in the editor
    let pin = use_selector(|s: &FrontState| s.map.current_pin.clone());
    // pin id last selected on the map (may be different)
    let selected_pin_id = use_selector(|s: &FrontState| s.map.selected_pin_id);
    let front_dispatch = Dispatch::<FrontState>::new();

    // show the input for entering a new list item
    let show_list_input = use_state(|| false);
    let show_list_input_onclick = {
        let show_list_input = show_list_input.clone();
        Callback::from(move |_| show_list_input.set(true))
    };

    // Editability
    let cancel_onclick = {
        let selected_pin_id = selected_pin_id.clone();
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            // just hit cancel -> close editor if it was a new pin
            if state.map.current_pin.id.is_none() {
                state.map.settings_tab = MapSettingsTab::None;
            } else if let Some(matching_pin) =
                find_matching_pin(&selected_pin_id)
            {
                // or revert the current pin data
                state.map.current_pin = matching_pin;
            }
            state.map.editable_pin = false;
        })
    };
    let marked_for_deletion = use_state(|| false);
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
            state.map.editable_pin = false;
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

    let icon_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            let mut s = elem.value();
            // Truncate to max number of unicode code points
            trunc_to_char(&mut s, ICON_MAX_CHARS);
            // Emoji might not be followed by variation selectors, so we also
            // truncate to a max number of graphemes
            trunc_to_grapheme(&mut s, ICON_MAX_GRAPHEMES);
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
            let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
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
                let mut s = elem.value();
                trunc_to_char(&mut s, LIST_MAX_CHARS);
                state.map.current_pin.lists.push(s);
                show_list_input.set(false);
            },
        )
    };

    // Tags
    let tags_elems = pin.tags.iter().enumerate().map(|(i, t)| {
        let remove_onclick = {
            front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
                state.map.current_pin.tags.remove(i);
            })
        };
        let update_key_onchange = front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlInputElement = e.target_dyn_into().unwrap();
                let mut s = elem.value();
                trunc_to_char(&mut s, TAG_KEY_MAX_CHARS);
                state.map.current_pin.tags[i].0 = s;
            },
        );
        let update_value_onchange = front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
                let mut s = elem.value();
                trunc_to_char(&mut s, TAG_VAL_MAX_CHARS);
                state.map.current_pin.tags[i].1 = s;
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
            state.map.current_pin.tags.push(("".into(), "".into()));
        });

    // Resize the map on any state change
    use_effect(move || {
        let event = web_sys::Event::new("resize").unwrap();
        web_sys::window().unwrap().dispatch_event(&event).unwrap();
    });
    html! {
        <div class="flex m-1">
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg \
            overflow-hidden px-4 py-2 flex flex-col gap-2 max-w-prose"
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
            <div class="flex items-center justify-between flex-wrap gap-2">
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
                    onclick={cancel_onclick}
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

/// Truncate to a given grapheme length.
///
/// Sometimes there is no "variation selector" after an emoji, so to prevent
/// putting too many emoji, we have to truncate just on graphemes
fn trunc_to_grapheme(s: &mut String, n: usize) {
    let upto = s
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .nth(n)
        .unwrap_or(s.len());
    s.truncate(upto);
}
