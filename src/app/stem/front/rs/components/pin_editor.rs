//! Component that shows the contents of a pin and allows editing of it.

use std::str::FromStr;

use common::{pin::Pin, state::MapSettingsTab, LngLat, ToBack, ToFront};
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::{
    components::confirm::Confirm,
    ui_state::{DerivedState, FrontState},
    web::clipboard::write_to_clipboard,
    websocket::{use_backend_event, WebsocketService},
};

static INPUT_STYLE: &str = "rounded bg-neutral-800 border border-neutral-700";
// prevent long words from increasing width of the element
static FREEFORM_TEXT_STYLE: &str =
    "whitespace-pre-wrap break-words table table-fixed w-full";

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
                    state.map.selected_pin_id = Some(*id);
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
///
/// The current_pin is set based on the selected_pin_id. A new pin should always
/// be opened with editable_pin=true and current_pin id and selected_pin_id set
/// to None.
///
/// The selected_pin_id is necessary, since there's a delay between saving a pin
/// and when the new id is assigned by the backend. We don't want the pin
/// details to flicker to the previous values for a moment before the backend
/// pin state updates with the new values.
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
            <span class="bg-neutral-800 px-2 py-1 rounded text-sm \
                         overflow-scroll"
            >
                {l.clone()}
            </span>
        }
    });

    // Tags
    let tags_elems = pin.tags.iter().map(|t| {
        let is_url = url::Url::parse(&t.1).is_ok();
        html! {
            <div class="flex items-center justify-start gap-2 select-text">
                <span class="w-1/3 overflow-scroll"> {t.0.clone()} </span>
                if is_url {
                    <a
                        class="w-2/3 max-h-32 overflow-scroll text-primary"
                        href={t.1.clone()}
                    >
                        <span class={format!(
                            "text-sm {}",
                            FREEFORM_TEXT_STYLE
                        )}>
                            {t.1.clone()}
                        </span>
                    </a>
                } else {
                    <div class="w-2/3 max-h-32 overflow-scroll">
                        <span class={format!(
                            "text-sm {}",
                            FREEFORM_TEXT_STYLE
                        )}>
                            {t.1.clone()}
                        </span>
                    </div>
                }
            </div>
        }
    });

    let close_settings_tab =
        front_dispatch.reduce_mut_callback(|state: &mut FrontState| {
            state.map.settings_tab = MapSettingsTab::None
        });

    let copy_worked = use_state(|| Option::<bool>::None);
    let copy_link_onclick = {
        let pin = pin.clone();
        let copy_worked = copy_worked.clone();
        Callback::from(move |_e: MouseEvent| {
            if let Ok(link) = pin.to_url() {
                let copy_worked = copy_worked.clone();
                write_to_clipboard(link.clone(), |worked| {
                    // show if it worked or not for one second
                    yew::platform::spawn_local(async move {
                        copy_worked.set(Some(worked));
                        yew::platform::time::sleep(
                            std::time::Duration::from_secs(1),
                        )
                        .await;
                        copy_worked.set(None);
                    });
                });
            }
        })
    };

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
        // 50lvh is 50% of the viewport's "large" height
        <div class="flex text-left max-h-[50lvh] overflow-scroll mx-1">
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg h-full \
            px-4 py-2 flex flex-col gap-2 max-w-prose my-1"
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
                <button
                    class="py-1 text-primary flex gap-2 items-center"
                    onclick={copy_link_onclick}
                >
                    {"Copy Scoria Link"}
                    if copy_worked.is_none() {
                        // default is a blue link
                        <Icon icon_id={IconId::BootstrapLink45Deg}
                            class="h-6 w-6" />
                    } else if *copy_worked == Some(true) {
                        <Icon icon_id={IconId::BootstrapCheck}
                            class="h-6 w-6 animate-inout" />
                    } else {
                        <Icon icon_id={IconId::BootstrapX}
                            class="h-6 w-6 text-red-500 animate-inout" />
                    }
                </button>
                <div></div>
            </div>
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
    // Save button saves changes or executes the deletion if th delete button is
    // slelected.
    let save_onclick = {
        let wss = use_context::<WebsocketService>().unwrap();
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            wss.send_msg(ToBack::SavePin(state.map.current_pin.clone()));
            state.map.editable_pin = false;
        })
    };

    let confirming_delete = use_state(|| false);
    let delete_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_| confirming_delete.set(true))
    };
    let cancel_delete_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_: MouseEvent| confirming_delete.set(false))
    };
    let ok_delete_onclick = {
        let wss = use_context::<WebsocketService>().unwrap();
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            if let Some(id) = state.map.current_pin.id {
                // has id, so was persisted to backend, delete it
                wss.send_msg(ToBack::DeletePin(id));
            }
            // clear the current pin
            state.map.current_pin = Default::default();
            state.map.selected_pin_id = None;
            state.map.editable_pin = false;
            state.map.settings_tab = MapSettingsTab::None;
        })
    };

    let icon_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            if state.map.current_pin.set_icon(elem.value()).is_err() {
                // invalid, reset to previous value
                elem.set_value(&state.map.current_pin.icon);
            }
        },
    );
    let name_onchange = front_dispatch.reduce_mut_callback_with(
        move |state: &mut FrontState, e: Event| {
            let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
            state.map.current_pin.set_name(elem.value());
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
            if state.map.current_pin.set_lnglat(newlnglat).is_err() {
                // invalid, revert
                elem.set_value(&state.map.current_pin.lnglat.to_string());
            }
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
                state.map.current_pin.push_list(elem.value());
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
                state.map.current_pin.update_key_at(i, elem.value());
            },
        );
        let update_value_onchange = front_dispatch.reduce_mut_callback_with(
            move |state: &mut FrontState, e: Event| {
                let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
                state.map.current_pin.update_val_at(i, elem.value());
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
        <div class="flex text-left max-h-[50lvh] mx-1 overflow-scroll">
        if *confirming_delete {
            <Confirm
                title={"Delete saved place?"}
                ok={ok_delete_onclick}
                cancel={cancel_delete_onclick}
            />
        }
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
            <div class="flex items-center justify-between flex-wrap gap-2">
                <button class="px-3 py-1.5 flex items-center rounded-lg \
                    bg-neutral-800 text-red-500"
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
                <button class="px-3 py-1.5 rounded-lg bg-neutral-800 \
                    text-primary"
                    onclick={save_onclick}
                >
                    {"Save"}
                </button>
            </div>
        </div>
        </div>
    }
}
