//! Controller for point selections.

use std::rc::Rc;

use common::mounted::{MountID, MAIN_DB_MOUNT_ID, MAIN_DB_NAME};
use common::popups::{PopUp, PopUpCode};
use common::time_range::TimeDeltaRange;
use common::ToFront;
use common::{state::MapSettingsTab, Location};
use jiff::Zoned;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::HtmlElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::confirm::Confirm;
use crate::components::time_range_picker::update_time_range;
use crate::components::Select;
use crate::unwrapping::unwrap_result_or_log;
use crate::websocket::{use_backend_event_with_deps, ToBack, WebsocketService};
use crate::{
    maplibre::{add_popup, binds::Map},
    ui_state::{BackState, FrontState},
};

/// Element id of the popup's Show Day button.
static SHOW_DAY_BTN_ID: &str = "popup_show_day_btn";

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

    let on_deletion_result = {
        let front_dispatch = front_dispatch.clone();
        move |msg: &ToFront| {
            if let ToFront::PopUp(PopUp {
                code: PopUpCode::DeletePoints,
                ..
            }) = msg
            {
                // clear selections after the deletion has completed in the
                // backend, and we receive the result popup
                front_dispatch.reduce_mut(|s: &mut FrontState| {
                    s.selected_points = vec![]
                });
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
            <p>
                {format!(
                    "Tap a point to select it. Points selected: {}",
                     n_selected,
                )}
            </p>
            <CopyPoints />
            <div class="flex justify-between">
                <button
                    class="px-3 py-1.5 rounded-lg bg-neutral-800 \
                        text-red-500 disabled:text-neutral-500 flex \
                        items-center gap-2"
                    onclick={delete_onclick}
                    disabled={delete_disabled}
                >
                    <Icon icon_id={IconId::BootstrapTrash} class="h-5 w-5" />
                    {"Delete Selected"}
                </button>
                <button class="px-3 py-1.5 text-primary rounded-lg \
                    bg-neutral-800"
                    onclick={clear_onclick}
                >
                    {"Unselect All"}
                </button>
            </div>
        </div>
        </div>
    }
}

#[function_component]
fn CopyPoints() -> Html {
    let n_selected = use_selector(|s: &FrontState| s.selected_points.len());
    let dest_db = use_selector(|s: &FrontState| s.copy_dest_db);
    let mounted_db_settings =
        use_selector(|s: &FrontState| s.mounted_db_settings.clone());
    let dest_db_name = mounted_db_settings
        .get(&dest_db)
        .map(|db| db.name.clone())
        .unwrap_or_else(|| MAIN_DB_NAME.to_string());

    let db_choices = mounted_db_settings
        .iter()
        .map(|(_id, db)| db.name.clone())
        .collect::<Vec<_>>();
    let front_dispatch = Dispatch::<FrontState>::new();
    let db_choice_onchange = front_dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, choice: String| {
            let choice_id = s
                .mounted_db_settings
                .iter()
                .find(|(_id, db)| db.name == choice)
                .map(|(id, _db)| *id)
                .unwrap_or(MAIN_DB_MOUNT_ID);
            s.copy_dest_db = choice_id;
        },
    );

    let wss = use_context::<WebsocketService>().unwrap();
    let confirm_message = format!(
        "Copy {n_selected} point{} to database \"{dest_db_name}\"?",
        if *n_selected > 1 { "s" } else { "" }
    );

    let confirming_copy = use_state(|| false);
    let copy_onclick = {
        let confirming_copy = confirming_copy.clone();
        Callback::from(move |_e: MouseEvent| {
            confirming_copy.set(true);
        })
    };
    let ok_onclick = {
        let confirming_copy = confirming_copy.clone();
        Callback::from(move |_: MouseEvent| {
            wss.send_msg(ToBack::CopySelectedLocationsToDatabase);
            confirming_copy.set(false)
        })
    };
    let cancel_onclick = {
        let confirming_copy = confirming_copy.clone();
        Callback::from(move |_: MouseEvent| confirming_copy.set(false))
    };
    let copy_disabled = mounted_db_settings.len() <= 1;

    let on_copy_result = {
        let front_dispatch = front_dispatch.clone();
        move |msg: &ToFront| {
            if let ToFront::PopUp(PopUp {
                code: PopUpCode::CopyPoints,
                ..
            }) = msg
            {
                // clear selections after the copy has completed in the backend,
                // and we receive the result popup
                front_dispatch.reduce_mut(|s: &mut FrontState| {
                    s.selected_points = vec![]
                });
            }
        }
    };
    use_backend_event_with_deps(on_copy_result, n_selected);
    html! {
        <>
            if *confirming_copy {
                <Confirm
                    title={confirm_message}
                    ok={ok_onclick}
                    cancel={cancel_onclick}
                />
            }
            <div class="flex justify-between items-center">
                <p> {"Copy to: "} </p>
                <Select<String>
                    selection={dest_db_name}
                    choices={db_choices}
                    onchange={db_choice_onchange}
                    class="mx-2 text-wrap w-[50vw] grow"
                    id="copy_destination_database"
                />
                <button
                    class="px-3 py-1.5 rounded-lg bg-neutral-800 \
                        text-primary disabled:text-neutral-500 flex \
                        items-center gap-2"
                    onclick={copy_onclick}
                    disabled={copy_disabled}
                >
                    {"Copy"}
                </button>
            </div>
        </>
    }
}

/// Handle receipt of a location from the backend either by displaying a popup
/// over it or selecting it. Returns the popup's Show Day click closure, which
/// the caller must keep alive as long as the popup can be clicked.
pub fn handle_nearest_location(
    map: Rc<Map>,
    mount_id: &MountID,
    loc: &Location,
    zdt: &Zoned,
) -> Option<Closure<dyn Fn()>> {
    let front_dispatch = Dispatch::<FrontState>::new();
    let front_state = front_dispatch.get();
    if front_state.map.settings_tab == MapSettingsTab::SelectPoints {
        front_dispatch.reduce_mut(|s: &mut FrontState| {
            if let Some(index) = s
                .selected_points
                .iter()
                .position(|p| p.0 == (*mount_id, loc.timestamp))
            {
                s.selected_points.remove(index);
            } else {
                s.selected_points
                    .push(((*mount_id, loc.timestamp), loc.lnglat()));
            }
        });
        None
    } else {
        let bg_color = front_state
            .map
            .popup_color
            .clone()
            .unwrap_or(front_state.map.style.solid_color.rgb.clone());
        let text = popup_text(loc, zdt);
        add_popup(map, &loc.lnglat(), &text, &bg_color);
        attach_show_day_onclick(zdt)
    }
}

/// Attach the click handler to the popup's Show Day button, which sets the
/// time range to the day containing the point, honoring the day separation
/// time.
fn attach_show_day_onclick(zdt: &Zoned) -> Option<Closure<dyn Fn()>> {
    let timestamp = zdt.timestamp();
    let onclick = Closure::wrap(Box::new(move || {
        let map_tz = Dispatch::<BackState>::new().get().map_tz.clone();
        Dispatch::<FrontState>::new().reduce_mut(|s: &mut FrontState| {
            s.map.time_delta_range = unwrap_result_or_log!(
                TimeDeltaRange::day_containing(timestamp)
            );
            update_time_range(s, &map_tz);
        });
    }) as Box<dyn Fn()>);
    let btn: HtmlElement = web_sys::window()?
        .document()?
        .get_element_by_id(SHOW_DAY_BTN_ID)?
        .dyn_into()
        .ok()?;
    btn.set_onclick(Some(onclick.as_ref().unchecked_ref()));
    Some(onclick)
}

pub fn popup_text(loc: &Location, zdt: &Zoned) -> String {
    let front_state = Dispatch::<FrontState>::new().get();
    let unit_pref = front_state.unit_pref;
    let time_pref = &front_state.time_pref;
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
        {}<br>\
        <button id=\"{SHOW_DAY_BTN_ID}\" \
        style=\"text-decoration: underline;\">Show Day</button>",
        latlon,
        accuracy_speed_course,
        alt,
        time_pref
            .format_jiff_datetime(zdt, &None)
            .unwrap_or_else(|_| "Time ?".to_string())
    )
}
