//! Global state manager for the UI. Yewdux is used to provide access to the
//! global state (Store trait).
//!
//! Note that if loading the UI for the first time directly at a non-Sense route
//! (such as Analyze), this will differ from the default of Sense, triggering a
//! write to the backend, which will overwrite whatever the backend saved from
//! the previous launch. This is only an issue during development, since in the
//! app the UI is always loaded at the splash page until the persisted state
//! arrives.

use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use common::{
    mounted::{MountID, MountedDB, MAIN_DB_MOUNT_ID, MAIN_DB_NAME},
    state::{MapState, PendingEvents},
    time_range::TimeDeltaRange,
};
use yewdux::prelude::*;

use crate::{
    router::Route,
    swift_poke,
    websocket::{Callback, ToBack, ToFront, WebsocketService},
};

/// Thin wrappers around the shared state definition. Implements yewdux's Store
/// trait. Dereferences to the internal common::{FrontState, BackState}.
#[derive(Debug, Clone, PartialEq, Store)]
pub struct FrontState(common::FrontState);

impl Default for FrontState {
    fn default() -> Self {
        Self(common::FrontState {
            map: MapState {
                // the real default for the time range. serde annotations just
                // pick values for cases when deserialization fails.
                time_delta_range: TimeDeltaRange::today(),
                time_range: TimeDeltaRange::today()
                    .to_time_range("UTC", jiff::civil::Time::MIN)
                    .unwrap(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, Store)]
pub struct BackState(common::BackState);

impl Deref for FrontState {
    type Target = common::FrontState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FrontState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for BackState {
    type Target = common::BackState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BackState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Default, Store)]
pub struct DerivedState(common::state::DerivedState);

impl Deref for DerivedState {
    type Target = common::state::DerivedState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DerivedState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Initialize the frontend state and handle pending events upon receiving the
/// ToFront::Startup websocket message.
pub fn init_state(
    front_state: &Option<common::FrontState>,
    back_state: &common::state::BackState,
    derived_state: &common::state::DerivedState,
    events: &Option<PendingEvents>,
) {
    do_init_state(front_state, back_state, derived_state);
    handle_pending_events(events);
    init_derived_listener();
}

/// Initialize the front state as received from the backend.
fn do_init_state(
    front_state: &Option<common::FrontState>,
    back_state: &common::state::BackState,
    derived_state: &common::state::DerivedState,
) {
    let front_dispatch = Dispatch::<FrontState>::new();
    if let Some(mut state) = front_state.clone() {
        // Update our "static" time_range to match the delta range
        // This way the selected time_range doesn't abruptly change on
        // the user as time passes while the app is open, but updates
        // between app launches
        if let Ok(new_time_range) = state.map.time_delta_range.to_time_range(
            &back_state.map_tz,
            state.time_pref.day_separation_time,
        ) {
            state.map.time_range = new_time_range;
        }
        // TODO: reset to today if they've been away for 1+ hour
        front_dispatch.reduce_mut(|s| **s = state);
    }
    Dispatch::<BackState>::new().reduce_mut(|s| **s = back_state.clone());
    Dispatch::<DerivedState>::new().reduce_mut(|s| **s = derived_state.clone());
}

/// Update state according to pending events received at initialization, or
/// during running.
///
/// Must be called after init_front_state. Setting the persisted route here
/// determines where the UI initially opens to. Also used for acting on events
/// that are received while the app is running, though for scoria:// links
/// clicked when the map is open this won't zoom to the right map marker since
/// no maplibre calls are made.
///
/// Can be called if events are pushed by the backend, in which case the return
/// value is the route to go to.
pub fn handle_pending_events(events: &Option<PendingEvents>) -> Option<Route> {
    let front_dispatch = Dispatch::<FrontState>::new();
    let Some(events) = events else {
        return None;
    };
    let Some(url) = &events.opened_url else {
        return None;
    };
    if let Ok(pin) = common::pin::Pin::from_url(url.clone()) {
        front_dispatch.reduce_mut(|s: &mut FrontState| {
            s.map.view_pos.center = pin.lnglat;
            s.map.view_pos.zoom = 16.0;
            s.map.current_pin = pin;
            s.map.selected_pin_id = None;
            s.map.editable_pin = true;
            s.map.settings_tab = common::state::MapSettingsTab::PinDetails;
            s.route = common::state::PersistedRoute::Analyze;
        });
        return Some(Route::Analyze);
    }
    None
}

// Updater for back and derived state, which is not sensitive to initialization
// order
pub fn get_update_callback() -> Callback {
    let back_dispatch = Dispatch::<BackState>::new();
    let derived_dispatch = Dispatch::<DerivedState>::new();
    let callback = move |msg: &ToFront| match msg {
        ToFront::Response(_, _) => (),
        ToFront::BackState(val) => {
            // update our known backend state
            back_dispatch.reduce_mut(|s| **s = val.clone())
        }
        ToFront::DerivedState(val) => {
            derived_dispatch.reduce_mut(|s| **s = val.clone())
        }
        ToFront::SwiftPoke => swift_poke::poke(),
        // These messages handled by other message listeners
        ToFront::Startup(..) => (),
        ToFront::PendingEvents(_) => (),
        ToFront::GeojsonUpdated => (),
        ToFront::NearestLocation(..) => (),
        ToFront::NewPinId(_) => (),
        ToFront::PopUp(_) => (),
    };
    Box::new(callback)
}

/// Send changes to the FrontState to the backend
struct FrontStateListener {
    pub wss: WebsocketService,
}
impl Listener for FrontStateListener {
    type Store = FrontState;

    fn on_change(&mut self, state: Rc<Self::Store>) {
        self.wss
            .send_msg(ToBack::SetFrontState(Box::new((**state).clone())));
    }
}

pub fn init_front_listener(wss: WebsocketService) {
    let state_listener = FrontStateListener { wss };
    init_listener(state_listener);
}

fn init_derived_listener() {
    // derived state may arrive before front state, so do a manual update
    let mut derived_listener = DerivedStateListener {
        previous_dbs_on_disk: Default::default(),
    };
    let derived_state = Dispatch::<DerivedState>::new().get();
    derived_listener.on_change(derived_state);
    init_listener(derived_listener);
}

/// This listener updates the front state with database settings. It listens for
/// changes to dbs_on_disk, and adds/removes dbs from the front state as needed.
struct DerivedStateListener {
    previous_dbs_on_disk: BTreeMap<MountID, Option<String>>,
}
impl Listener for DerivedStateListener {
    type Store = DerivedState;

    fn on_change(&mut self, state: Rc<Self::Store>) {
        let dbs_on_disk = &state.mounted_dbs_on_disk;
        let last_mounted = &state.last_mounted;
        if self.previous_dbs_on_disk == *dbs_on_disk {
            return;
        }
        self.previous_dbs_on_disk = dbs_on_disk.clone();
        let dispatch = Dispatch::<FrontState>::new();
        let mounted_db_settings = &dispatch.get().mounted_db_settings;
        // if there's a database on disk that's not in the front state, add it
        for id in dbs_on_disk.keys() {
            if mounted_db_settings.get(id).is_none() {
                // not listed in the named databases in the front state, add it
                let name = match last_mounted {
                    // use fname as name if it's the last db mounted
                    Some((last_id, name)) if last_id == id => name.clone(),
                    _ if *id == MAIN_DB_MOUNT_ID => MAIN_DB_NAME.to_string(),
                    _ => "?".to_string(),
                };
                dispatch.reduce_mut(|s| {
                    s.mounted_db_settings.insert(
                        *id,
                        MountedDB {
                            name,
                            enabled: true,
                        },
                    )
                });
            }
        }
        // if a db is in the front state but not on disk, remove it
        for id in mounted_db_settings.keys() {
            if dbs_on_disk.get(id).is_none() {
                // database no longer on disk, remove from ui state
                dispatch.reduce_mut(|s| {
                    s.mounted_db_settings.remove(id);
                });
            }
        }
    }
}
