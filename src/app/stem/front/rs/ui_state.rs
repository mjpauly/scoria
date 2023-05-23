//! Global state manager for the UI. Yewdux is used to provide access to the
//! global state (Store trait).

use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

use yewdux::prelude::*;

use crate::websocket::{Callback, ToBack, ToFront, WebsocketService};

/// Thin wrappers around the shared state definition. Implements yewdux's Store
/// trait. Dereferences to the internal common::{FrontState, BackState}.
#[derive(Debug, Clone, PartialEq, Default, Store)]
pub struct FrontState(common::FrontState);

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
            front_dispatch.reduce_mut(|s| **s = val.clone())
        }
        ToFront::BackState(val) => {
            // update our known backend state
            back_dispatch.reduce_mut(|s| **s = val.clone())
        }
        // This message handled by other callbacks, and not stored globally
        ToFront::LocationTimeRange(..) => (),
        ToFront::LastLocation(..) => (),
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
