//! Sends update messages to components that subscribe for updates from the
//! backend. Each component subscribes to one or multiple message types.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use yew::prelude::*;

thread_local!(static EB: RefCell<EventBus> = RefCell::new(EventBus::new()));

/// Subscribe to messages from the websocket
pub fn subscribe(cb: Callback<()>) {
    EB.with(|e| {
        (*e.borrow_mut()).subscribe(cb);
    });
}

/// For use by websocket to send messages through the event bus
pub fn send(msg: Request) {
    EB.with(|e| {
        (*e.borrow()).send(msg);
    });
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    EventBusMsg(String),
}

struct EventBus {
    subscribers: Vec<Callback<()>>,
}

impl EventBus {
    fn new() -> Self {
        log::debug!("EventBus created.");
        Self {
            subscribers: Vec::new(),
        }
    }

    /// Called by components to subscribe to messages
    fn subscribe(&mut self, cb: Callback<()>) {
        self.subscribers.push(cb);
    }

    /// Called by the websocket handler to pass messages to EventBus
    fn send(&self, msg: Request) {
        match msg {
            Request::EventBusMsg(s) => {
                log::debug!("event bus got message: {}", s);
                for sub in self.subscribers.iter() {
                    // scope.respond(*sub, s.clone())
                    (*sub).emit(());
                }
            }
        }
    }
}
