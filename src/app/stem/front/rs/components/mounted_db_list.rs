//! The list of currently mounted databases.

use common::mounted::{MountID, MountedDB, MAIN_DB_MOUNT_ID};
use common::state::DbStatus;
use common::ToBack;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::confirm::ConfirmAction;
use crate::components::ShortInput;
use crate::components::{
    ErrorMessage, SettingsCard, ToggleSwitch, SECONDARY_BUTTON_STYLE,
};
use crate::swift_poke;
use crate::ui_state::{DerivedState, FrontState};
use crate::websocket::WebsocketService;

#[function_component]
pub fn MountedDBList() -> Html {
    let dbs_on_disk =
        use_selector(|s: &DerivedState| s.mounted_dbs_on_disk.clone());
    let mounted_db_settings =
        use_selector(|s: &FrontState| s.mounted_db_settings.clone());
    let editing = use_state(|| false);
    let mounted_html = mounted_db_settings.iter().map(|(id, db)| {
        let status = dbs_on_disk.get(id).cloned();
        html! {
            if *id == MAIN_DB_MOUNT_ID {
                <MainDBSetting db={db.clone()} status={status}/>
            } else {
                <MountedDBSetting
                    id={*id}
                    db={db.clone()}
                    editing={*editing}
                    status={status}
                />
            }
        }
    });
    let wss = use_context::<WebsocketService>().unwrap();
    let mount_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::MountDB);
        swift_poke::poke();
    });
    let edit_onclick = {
        let editing = editing.clone();
        Callback::from(move |_e| {
            editing.set(!*editing);
        })
    };
    html! {
        <>
            if !mounted_db_settings.is_empty() {
                <div class="flex items-center justify-between mt-4 \
                    text-neutral-500 px-4"
                >
                    <span>{"Name"}</span>
                    <span>{"Enabled"}</span>
                </div>
            }

            <SettingsCard class="mt-1">
                {for mounted_html}
            </SettingsCard>

            <div class="flex items-center justify-between mt-2">
                <button
                    class={format!("px-3 py-1.5 {}", SECONDARY_BUTTON_STYLE)}
                    onclick={mount_onclick}
                >
                    {"Mount Database"}
                </button>
                if mounted_db_settings.len() > 1 {
                    <button
                        class={format!(
                            "px-3 py-1.5 {}",
                            SECONDARY_BUTTON_STYLE
                        )}
                        onclick={edit_onclick}
                    >
                        if *editing {
                            {"Done"}
                        } else {
                            {"Edit"}
                        }
                    </button>
                }
            </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct MountedDBSettingProps {
    pub id: MountID,
    pub db: MountedDB,
    pub status: Option<DbStatus>, // None if not found on disk
    pub editing: bool,
}

/// Settings for a mounted database, allowing for editing the name,
/// enabling/disabling the database, and deleting it.
#[function_component]
pub fn MountedDBSetting(p: &MountedDBSettingProps) -> Html {
    let id = p.id;
    let db = &p.db;
    let dispatch = Dispatch::<FrontState>::new();
    let onclick = dispatch.reduce_mut_callback(move |s| {
        if let Some(setting) = s.mounted_db_settings.get_mut(&id) {
            setting.enabled = !setting.enabled;
        }
    });
    let name_onchange =
        dispatch.reduce_mut_callback_with(move |s, new_name| {
            if let Some(setting) = s.mounted_db_settings.get_mut(&id) {
                setting.name = new_name;
            }
        });
    let confirming_delete = use_state(|| false);
    let delete_onclick = {
        let confirming_delete = confirming_delete.clone();
        Callback::from(move |_e: MouseEvent| {
            confirming_delete.set(true);
        })
    };
    let wss = use_context::<WebsocketService>().unwrap();
    let do_delete = Callback::from(move |()| {
        wss.send_msg(ToBack::DeleteMountedDB(id));
    });
    let confirm_message = format!("Delete database \"{}\"?", db.name);
    html! {
        <div class="flex flex-col gap-2 py-2 px-4 w-full active:bg-neutral-800">
            <div class="flex items-center justify-between w-full">
                <ConfirmAction
                    title={confirm_message}
                    visible={confirming_delete}
                    callback={do_delete}
                />
                if p.editing {
                    <ShortInput
                        value={db.name.clone()}
                        onchange={name_onchange}
                    />
                } else {
                    <span class="text-left">{&db.name}</span>
                }
                <StatusIcon status={p.status.clone()} />
                if p.editing {
                    <button onclick={delete_onclick}>
                        <Icon icon_id={IconId::BootstrapTrash}
                            class="text-neutral-400"/>
                    </button>
                } else {
                    <ToggleSwitch
                        checked={db.enabled}
                        onclick={onclick}
                    />
                }
            </div>
            <StatusDetail status={p.status.clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct MainDBSettingProps {
    pub db: MountedDB,
    pub status: Option<DbStatus>,
}

/// Setting to enable/disable the main database. Cannot be deleted, since that
/// has to be done by deleting the app
#[function_component]
pub fn MainDBSetting(p: &MainDBSettingProps) -> Html {
    let db = &p.db;
    let dispatch = Dispatch::<FrontState>::new();
    let onclick = dispatch.reduce_mut_callback(move |s| {
        if let Some(setting) = s.mounted_db_settings.get_mut(&MAIN_DB_MOUNT_ID)
        {
            setting.enabled = !setting.enabled;
        }
    });
    html! {
        <div class="flex flex-col gap-2 py-2 px-4 w-full active:bg-neutral-800">
            <div class="flex items-center justify-between w-full">
                <span class="italic">{&db.name}</span>
                <StatusIcon status={p.status.clone()} />
                <ToggleSwitch
                    checked={db.enabled}
                    onclick={onclick}
                />
            </div>
            <StatusDetail status={p.status.clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct StatusProps {
    pub status: Option<DbStatus>,
}

/// Icon next to the name: a warning on error, a spinner while opening.
#[function_component]
fn StatusIcon(p: &StatusProps) -> Html {
    match &p.status {
        Some(DbStatus::Error(_)) => html! {
            <Icon icon_id={IconId::BootstrapExclamationTriangle}
                class="text-red-500" />
        },
        Some(DbStatus::Opening { .. }) => html! {
            <div class="h-4 w-4 rounded-full border-2 border-neutral-400 \
                border-t-transparent animate-spin" />
        },
        _ => html! {},
    }
}

/// Detail line under the name: the error, or the opening stage.
#[function_component]
fn StatusDetail(p: &StatusProps) -> Html {
    match &p.status {
        Some(DbStatus::Error(e)) => html! {
            <ErrorMessage>
                <span class="whitespace-pre-wrap">{e}</span>
            </ErrorMessage>
        },
        Some(DbStatus::Opening { stage, progress }) => html! {
            <span class="text-sm text-neutral-400">
                {opening_text(stage, *progress)}
            </span>
        },
        _ => html! {},
    }
}

/// "Loading: stage 42%" text for an opening database.
pub fn opening_text(stage: &str, progress: Option<f32>) -> String {
    match progress {
        Some(p) => format!("Loading: {stage} {:.0}%", p * 100.),
        None => format!("Loading: {stage}"),
    }
}
