//! Global state manager for the UI

use crate::websocket::{Callback, ToFront};
use std::cell::RefCell;
use std::rc::Rc;

// #[derive(Clone, PartialEq)]
#[derive(Clone)]
pub struct UIState {
    pub location_is_enabled: Rc<RefCell<bool>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            location_is_enabled: Rc::new(RefCell::new(true)),
        }
    }

    pub fn get_update_callback(&self) -> Callback {
        let self_clone = self.clone();
        let callback = move |msg: &ToFront| match msg {
            ToFront::LocationEnabled(val) => {
                *self_clone.location_is_enabled.borrow_mut() = *val
            }
            ToFront::Ping => todo!(),
            ToFront::Data(u8) => todo!(),
        };
        Box::new(callback)
    }
}

impl PartialEq for UIState {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}
