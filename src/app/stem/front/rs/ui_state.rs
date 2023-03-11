//! Global state manager for the UI

use crate::common::Location;
use crate::websocket::{Callback, ToFront};
use std::cell::RefCell;
use std::rc::Rc;

// #[derive(Clone, PartialEq)]
#[derive(Clone)]
pub struct UIState {
    pub location_is_enabled: Rc<RefCell<bool>>,
    pub distance_filter: Rc<RefCell<f32>>,
    pub last_location: Rc<RefCell<Option<Location>>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            location_is_enabled: Rc::new(RefCell::new(true)),
            distance_filter: Rc::new(RefCell::new(5.0)),
            last_location: Rc::new(RefCell::new(None)),
        }
    }

    pub fn get_update_callback(&self) -> Callback {
        let self_clone = self.clone();
        let callback = move |msg: &ToFront| match msg {
            ToFront::LocationEnabled(val) => {
                *self_clone.location_is_enabled.borrow_mut() = *val
            }
            ToFront::LastLocation(val) => {
                *self_clone.last_location.borrow_mut() = Some(val.clone())
            }
        };
        Box::new(callback)
    }
}

#[allow(unused_variables)]
impl PartialEq for UIState {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}
