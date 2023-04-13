//! Global state manager for the UI

use crate::common::{Location, LocationAccuracyMode};
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
    pub significant_changes: Rc<RefCell<bool>>,
    pub location_accuracy_mode: Rc<RefCell<LocationAccuracyMode>>,

    // Whether to use epsln tile server
    pub use_epsln_tile_server: Rc<RefCell<bool>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            last_location: Rc::new(RefCell::new(None)),
            locations_past_hour: Rc::new(RefCell::new(None)),

            location_is_enabled: Rc::new(RefCell::new(false)),
            distance_filter: Rc::new(RefCell::new(5.0)),
            significant_changes: Rc::new(RefCell::new(false)),
            location_accuracy_mode: Rc::new(RefCell::new(
                LocationAccuracyMode::Best,
            )),

            use_epsln_tile_server: Rc::new(RefCell::new(false)),
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
            ToFront::SignificantChanges(val) => {
                *self_clone.significant_changes.borrow_mut() = *val
            }
            ToFront::LocationAccuracyMode(val) => {
                *self_clone.location_accuracy_mode.borrow_mut() = *val
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
