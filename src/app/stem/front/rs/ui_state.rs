//! Global state manager for the UI

use crate::common::Location;
use crate::websocket::{Callback, ToFront};
use std::cell::RefCell;
use std::rc::Rc;

// #[derive(Clone, PartialEq)]
#[derive(Clone)]
pub struct UIState {
    // Location state
    pub last_location: Rc<RefCell<Option<Location>>>,
    pub locations_past_hour: Rc<RefCell<Option<i32>>>,

    // Location configuration
    pub location_is_enabled: Rc<RefCell<bool>>,
    pub distance_filter: Rc<RefCell<f32>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            last_location: Rc::new(RefCell::new(None)),
            locations_past_hour: Rc::new(RefCell::new(None)),

            location_is_enabled: Rc::new(RefCell::new(true)),
            distance_filter: Rc::new(RefCell::new(5.0)),
        }
    }

    pub fn get_update_callback(&self) -> Callback {
        let self_clone = self.clone();
        let callback = move |msg: &ToFront| match msg {
            ToFront::LastLocation(val) => {
                *self_clone.last_location.borrow_mut() = Some(val.clone())
            }
            ToFront::LocationsPastHour(val) => {
                *self_clone.locations_past_hour.borrow_mut() = Some(*val)
            }
            ToFront::LocationEnabled(val) => {
                *self_clone.location_is_enabled.borrow_mut() = *val
            }
            ToFront::DistFilt(val) => {
                *self_clone.distance_filter.borrow_mut() = *val
            }
            // This message handled by some other callback, and not stored here
            ToFront::LocationTimeRange(..) => (),
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
