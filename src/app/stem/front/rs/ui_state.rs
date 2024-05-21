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
    ops::{Deref, DerefMut},
    rc::Rc,
};

use common::state::MapState;
use yewdux::prelude::*;

use crate::{
    components::time_range_picker::{local_offset, time_delta_range_today},
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
                // timezone-aware time_delta_range, which is preferred over the
                // default implementation in common::state, which is a backup
                // the backend can run if deserialization fails.
                time_delta_range: time_delta_range_today(),
                time_range: (&time_delta_range_today()).into(),
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

pub fn get_update_callback() -> Callback {
    let front_dispatch = Dispatch::<FrontState>::new();
    // logic for setting the front state when it's received from the backend
    let set_front_state = move |mut state: common::FrontState| {
        if state.map.time_delta_range.offset.is_none() {
            // Backend failed to deserialize -> set to correct offset
            state.map.time_delta_range = time_delta_range_today();
        }
        // Always ensure the UTC offset is up-to-date
        state.map.time_delta_range.offset = Some(local_offset());
        // Update our "static" time_range to match the delta range
        // This way the selected time_range doesn't abruptly change on
        // the user as time passes while the app is open, but updates
        // between app launches
        // TODO: reset to today if they've been away for 1+ hour
        state.map.time_range = (&state.map.time_delta_range).into();
        front_dispatch.reduce_mut(|s| **s = state)
    };

    let back_dispatch = Dispatch::<BackState>::new();
    let derived_dispatch = Dispatch::<DerivedState>::new();
    let callback = move |msg: &ToFront| match msg {
        ToFront::FrontState(val) => {
            // we get the FrontState at startup
            if let Some(state) = val {
                set_front_state(state.clone());
            }
        }
        ToFront::BackState(val) => {
            // update our known backend state
            back_dispatch.reduce_mut(|s| **s = val.clone())
        }
        ToFront::DerivedState(val) => {
            derived_dispatch.reduce_mut(|s| **s = val.clone())
        }
        // These messages handled by other callbacks, and not stored globally
        ToFront::GeojsonUpdated => (),
        ToFront::PopupText { .. } => (),
        ToFront::NewPinId(_) => (),
        ToFront::SwiftPoke => swift_poke::poke(),
    };
    Box::new(callback)
}

/// Send changes to the FrontState to the backend
struct StateListener {
    pub wss: WebsocketService,
}
impl Listener for StateListener {
    type Store = FrontState;

    fn on_change(&mut self, state: Rc<Self::Store>) {
        self.wss
            .send_msg(ToBack::SetFrontState(Box::new((**state).clone())));
    }
}

pub fn init_backend_listener(wss: WebsocketService) {
    let state_listener = StateListener { wss };
    init_listener(state_listener);
}
