//! Controller for point selections.

use std::rc::Rc;

use common::ToFront;
use common::{state::MapSettingsTab, Location};
use yew::prelude::*;
// use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use super::time_range_picker::local_offset;
use crate::components::confirm::Confirm;
use crate::components::{ErrorMessage, SuccessMessage};
use crate::websocket::{use_backend_event_with_deps, ToBack, WebsocketService};
use crate::{
    maplibre::{add_popup, binds::Map},
    ui_state::FrontState,
};

#[function_component]
pub fn SelectPointsControl() -> Html {
    let selected_points =
        use_selector(|s: &FrontState| s.selected_points.clone());
    let n_selected = selected_points.len();

    let front_dispatch = Dispatch::<FrontState>::new();
    let clear_onclick =
        front_dispatch.reduce_mut_callback(move |state: &mut FrontState| {
            state.selected_points = vec![];
        });

    let wss = use_context::<WebsocketService>().unwrap();
    let confirm_message = format!(
        "Delete {n_selected} point{}?",
        if n_selected > 1 { "s" } else { "" }
    );
    let confirming_delete = use_state(|| false);
    let delete_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_e: MouseEvent| {
            confirming_delete.set(true);
        })
    };
    let ok_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_: MouseEvent| {
            wss.send_msg(ToBack::DeleteSelectedLocations);
            confirming_delete.set(false)
        })
    };
    let cancel_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_: MouseEvent| confirming_delete.set(false))
    };
    let delete_disabled = selected_points.is_empty();

    let deletion_result = use_state(|| Option::<Html>::None);
    let on_deletion_result = {
        let deletion_result = deletion_result.clone();
        let front_dispatch = front_dispatch.clone();
        move |msg: &ToFront| {
            if let ToFront::DeleteLocationsResult(n_deleted) = msg {
                let clear_selected = || {
                    front_dispatch.reduce_mut(|s: &mut FrontState| {
                        s.selected_points = vec![]
                    })
                };
                if *n_deleted == n_selected as u64 {
                    // display sucess
                    deletion_result.set(Some(html! {
                        <SuccessMessage>
                            <span>
                                {format!(
                                    "Successfully deleted {n_deleted} points."
                                )}
                            </span>
                        </SuccessMessage>
                    }));
                    clear_selected();
                } else if *n_deleted > 0 {
                    // display error with partial success
                    deletion_result.set(Some(html! {
                        <ErrorMessage>
                            <span>
                                {format!(
                                    "Failed to delete points. Deleted \
                                    {n_deleted} of {n_selected}.",
                                )}
                            </span>
                        </ErrorMessage>
                    }));
                    clear_selected();
                } else {
                    // display error, don't reset selection
                    deletion_result.set(Some(html! {
                        <ErrorMessage>
                            <span>
                                {"Failed to delete points."}
                            </span>
                        </ErrorMessage>
                    }));
                }
            }
        }
    };
    use_backend_event_with_deps(on_deletion_result, n_selected);
    html! {
        <div class="flex m-1 text-left">
        if *confirming_delete {
            <Confirm
                title={confirm_message}
                ok={ok_onclick}
                cancel={cancel_onclick}
            />
        }
        <div class="flex-grow mx-auto bg-neutral-900 rounded-lg max-w-prose \
            text-left px-4 py-2 flex flex-col gap-2"
        >
            {(*deletion_result).clone()}
            <p>
                {format!(
                    "Tap a point to select it. Points selected: {}",
                     n_selected,
                )}
            </p>
            <div class="flex justify-between">
                <button class="px-3 py-1.5 text-primary rounded-lg \
                    bg-neutral-800"
                    onclick={clear_onclick}
                >
                    {"Unselect All"}
                </button>
                <button
                    class={"px-3 py-1.5 rounded-lg bg-neutral-800 \
                        text-red-500 disabled:text-neutral-500"}
                    onclick={delete_onclick}
                    disabled={delete_disabled}
                >
                    {"Delete Selected"}
                </button>
            </div>
        </div>
        </div>
    }
}

/// Handle receipt of a location from the backend either by displaying a popup
/// over it or selecting it.
pub fn handle_nearest_location(map: Rc<Map>, loc: &Location) {
    let front_dispatch = Dispatch::<FrontState>::new();
    let front_state = front_dispatch.get();
    if front_state.map.settings_tab == MapSettingsTab::SelectPoints {
        front_dispatch.reduce_mut(|s: &mut FrontState| {
            if let Some(index) =
                s.selected_points.iter().position(|p| p.0 == loc.timestamp)
            {
                s.selected_points.remove(index);
            } else {
                s.selected_points.push((loc.timestamp, loc.lnglat()));
            }
        })
    } else {
        let bg_color = front_state
            .map
            .popup_color
            .clone()
            .unwrap_or(front_state.map.style.solid_color.rgb.clone());
        let text = popup_text(loc);
        add_popup(map, &loc.lnglat(), &text, &bg_color);
    }
}

pub fn popup_text(loc: &Location) -> String {
    let front_state = Dispatch::<FrontState>::new().get();
    let unit_pref = front_state.unit_pref;
    let offset = local_offset();
    let time_pref = front_state.time_pref;
    let latlon = format!(
        "{}, {}",
        unit_pref.format_angle(loc.latitude, Some(6)),
        unit_pref.format_angle(loc.longitude, Some(6))
    );
    let mut accuracy_speed_course = format!(
        "±{}",
        unit_pref.format_small_length(loc.horizontal_accuracy, Some(2)),
    );

    if let Some(speed) = loc.speed {
        accuracy_speed_course +=
            &format!(", {}", unit_pref.format_velocity(speed, Some(2)));
    }
    if let Some(course) = loc.course {
        accuracy_speed_course +=
            &format!(", {}", unit_pref.format_angle(course, Some(2)));
    }

    let mut alt = loc.msl_altitude.map(|alt| {
        format!("{} altitude", unit_pref.format_small_length(alt, Some(2)))
    });
    if let Some(v_acc) = loc.vertical_accuracy {
        alt = alt.map(|alt| {
            format!("{alt} ±{}", unit_pref.format_small_length(v_acc, Some(2)))
        });
    }
    if let Some(story) = loc.story {
        alt = alt.map(|alt| format!("{alt}, story {story}"));
    }
    let alt = alt.map(|alt| format!("{alt}<br>")).unwrap_or_default();
    format!(
        "{}<br>\
        {}<br>\
        {}\
        {}",
        latlon,
        accuracy_speed_course,
        alt,
        time_pref
            .format_datetime(loc.timestamp.to_offset(offset))
            .unwrap_or_else(|_| "Time ?".to_string())
    )
}
