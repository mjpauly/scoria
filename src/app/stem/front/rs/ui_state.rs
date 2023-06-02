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
    components::time_range_picker::time_range_today,
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
                // timezone-aware time_range, which is preferred over the
                // default implementation in common::state, which is a backup
                // the backend can run if deserialization fails.
                time_range: time_range_today(),
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

pub fn get_update_callback() -> Callback {
    let front_dispatch = Dispatch::<FrontState>::new();
    let back_dispatch = Dispatch::<BackState>::new();
    let callback = move |msg: &ToFront| match msg {
        ToFront::FrontState(val) => {
            // we get the FrontState at startup
            if let Some(state) = val {
                front_dispatch.reduce_mut(|s| **s = state.clone())
            }
        }
        ToFront::BackState(val) => {
            // update our known backend state
            back_dispatch.reduce_mut(|s| **s = val.clone())
        }
        // These messages handled by other callbacks, and not stored globally
        ToFront::GeojsonUpdated => (),
        ToFront::PopupText { .. } => (),
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
        self.wss.send_msg(ToBack::SetFrontState((**state).clone()));
    }
}

pub fn init_backend_listener(wss: WebsocketService) {
    let state_listener = StateListener { wss };
    init_listener(state_listener);
}
