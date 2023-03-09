//! Connects to the backend via a websocket.
//!
//! Based on:
//! https://github.com/jtordgeman/YewChat/blob/websockets-part2/src/services/websocket.rs

use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// Re-export the message types
pub use crate::common::{MsgForBackend, MsgForFrontend};
use crate::{event_bus, event_bus::Request};

/// Shared WebsocketService instance.
#[derive(Clone)]
pub struct WebsocketService {
    // Rc provides immutable pointer cloning, RefCell provides runtime-checked
    // interior mutability for sending messages.

    // handle for sending a message to the backend through the websocket
    tx: Rc<RefCell<Sender<String>>>,

    // list of component subscribers to update on message received from backend
    subscribers: Rc<RefCell<Vec<Callback<()>>>>,
}

impl WebsocketService {
    /// Send a message to the websocket
    pub fn send_msg(&self, msg: String) {
        self.tx.borrow_mut().try_send(msg).unwrap();
    }

    /// Subscribe to messages from the backend
    pub fn subscribe(&self, cb: Callback<()>) {
        self.subscribers.borrow_mut().push(cb);
    }

    pub fn new() -> Self {
        let ws = WebSocket::open("ws://127.0.0.1:8081/ws").unwrap();

        let (mut write, mut read) = ws.split();

        // components write tx, and WebsocketService receives on rx
        let (in_tx, mut in_rx) =
            futures::channel::mpsc::channel::<String>(1000);

        spawn_local(async move {
            while let Some(s) = in_rx.next().await {
                // log::debug!("got event from channel! {}", s);
                write.send(Message::Text(s)).await.unwrap();
            }
        });

        let subscribers = Rc::new(RefCell::new(Vec::<Callback<()>>::new()));

        let subscribers_handle = subscribers.clone();
        spawn_local(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        log::debug!("from websocket: {}", data);
                        // event_bus::send(Request::EventBusMsg(data));
                        // notify component subscribers of the message
                        for sub in subscribers_handle.borrow().iter() {
                            (*sub).emit(());
                        }
                    }
                    Ok(Message::Bytes(b)) => {
                        let decoded = std::str::from_utf8(&b);
                        if let Ok(val) = decoded {
                            log::debug!("from websocket: {}", val);
                            event_bus::send(Request::EventBusMsg(val.into()));
                        }
                    }
                    Err(e) => {
                        log::error!("ws: {:?}", e)
                    }
                }
            }
            log::debug!("WebSocket Closed");
        });

        Self {
            tx: Rc::new(RefCell::new(in_tx)),
            subscribers,
        }
    }
}

impl PartialEq for WebsocketService {
    /// Yew uses PartialEq to determine if properties for a component have
    /// changed. Since we don't care about the interior state of the service, we
    /// just compare the smart pointers.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.tx, &other.tx)
            && Rc::ptr_eq(&self.subscribers, &other.subscribers)
    }
}
