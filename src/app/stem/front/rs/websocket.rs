//! Connects to the backend via a websocket.
//!
//! Callbacks are ordinary closures which take a &ToFront as an argument
//! and return nothing. They should check the message type using `if let` so
//! they only receive messages they are interested in. If they want to persist
//! the message data, they must clone it.
//!
//! Usage example within a Yew function component:
//!
//! ```
//! let wss = use_context::<WebsocketService>().unwrap();
//! let counter = use_state(|| 0);  // warning: always starts at 0
//! let on_sock_msg = {
//!     let counter = counter.clone();
//!     move |msg: &ToFront| {
//!         if let ToFront::Data(val) = msg {
//!             counter.set(*val);
//!         }
//!     }
//! };
//! wss.subscribe(Box::new(on_sock_msg));
//! ```
//!
//! ```
//! // Get a clone of the websocket service so we can TX/RX backend messages
//! let wss = use_context::<WebsocketService>().unwrap();
//! // Get a clone of the app state, so we initialize our local state correctly
//! let state = use_context::<UIState>().unwrap();
//!
//! // Get the location_is_enabled state from the UI's local copy.
//! // We store it in use_state_eq so a click on the checkbox can immediately
//! // refresh this component, and if we get an update from the backend it is
//! // only refreshed if the new state is different.
//! let location_is_enabled =
//!     use_state_eq(|| *state.location_is_enabled.borrow());
//!
//! // Subscribe to backend updates to the location enabled state.
//! // Only needed if we expect the backend to change this state without user
//! // input.
//! let on_backend_msg = {
//!     let location_is_enabled = location_is_enabled.clone();
//!     move |msg: &ToFront| {
//!         if let ToFront::LocationEnabled(val) = msg {
//!             location_is_enabled.set(*val);
//!         }
//!     }
//! };
//! // Generate a unique ID which doesn't change between renders since no deps
//! // are given to use_memo
//! let id = use_memo(|_| uuid::Uuid::new_v4(), ());
//! wss.subscribe(*id, Box::new(on_backend_msg));
//!
//! // Update our state on click and tell the backend. Telling the backend is
//! // only needed if the backend needs to know about this state change.
//! let on_click = {
//!     let location_is_enabled = location_is_enabled.clone();
//!     Callback::from(move |_e: MouseEvent| {
//!         let new_val = !*location_is_enabled;
//!         location_is_enabled.set(new_val);
//!         wss.send_msg(ToBack::SetLocationEnabled(new_val));
//!     })
//! };
//! ```
//!
//! The implementation is based on Rc<RefCell<>> so it is not thread safe.
//!
//! Based on:
//! https://github.com/jtordgeman/YewChat/blob/websockets-part2/src/services/websocket.rs

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use futures::channel::mpsc::{Receiver, Sender};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

// Re-export the message types
pub use crate::common::{ToBack, ToFront};

pub type Callback = Box<dyn Fn(&ToFront)>;

/// Shared WebsocketService instance.
#[derive(Clone)]
pub struct WebsocketService {
    // Rc provides immutable pointer cloning, RefCell provides runtime-checked
    // interior mutability for sending messages.

    // handle for sending a message to the backend through the websocket
    tx: Rc<RefCell<Sender<ToBack>>>,

    // list of component subscribers to update on message received from backend
    subscribers: Rc<RefCell<HashMap<Uuid, Callback>>>,
}

impl WebsocketService {
    /// Send a message to the websocket
    pub fn send_msg(&self, msg: ToBack) {
        self.tx.borrow_mut().try_send(msg).unwrap();
    }

    /// Subscribe to messages from the backend
    /// ```
    /// let id = use_memo(|_| uuid::Uuid::new_v4(), ());
    /// wss.subscribe(*id, Box::new(on_backend_msg));
    /// ```
    pub fn subscribe(&self, id: Uuid, cb: Callback) {
        self.subscribers.borrow_mut().insert(id, cb);
        // log::debug!("subscriber len: {}", self.subscribers.borrow().len());
    }

    pub fn new() -> Self {
        // Get the port that we connected to on the server
        let port = web_sys::window()
            .unwrap()
            .location()
            .port()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let address = format!("ws://127.0.0.1:{}/ws", port);
        log::debug!("Binding to websocket at {}", address);

        let ws = WebSocket::open(&address).unwrap();

        let (ws_write, ws_read) = ws.split();

        // components write yew_tx, and WebsocketService receives on rx
        let (yew_tx, yew_rx) = futures::channel::mpsc::channel::<ToBack>(1000);

        let subscribers =
            Rc::new(RefCell::new(HashMap::<Uuid, Callback>::new()));

        Self::spawn_websocket_reader(ws_read, subscribers.clone());
        Self::spawn_websocket_writer(yew_rx, ws_write);

        Self {
            tx: Rc::new(RefCell::new(yew_tx)),
            subscribers,
        }
    }

    /// Spawn the future that gets data from the mpsc channel and sends it
    /// through the websocket to the backend.
    fn spawn_websocket_writer(
        mut yew_rx: Receiver<ToBack>,
        mut ws_write: SplitSink<WebSocket, Message>,
    ) {
        spawn_local(async move {
            while let Some(msg) = yew_rx.next().await {
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                ws_write.send(Message::Bytes(encoded)).await.unwrap();
            }
        });
    }

    /// Spawn the future that reads data from the websocket and passes the
    /// messages to component subscribers
    fn spawn_websocket_reader(
        // Websocket stream we can read messages from
        mut ws_read: SplitStream<WebSocket>,
        // Handle to the subscriberes to notify
        subscribers: Rc<RefCell<HashMap<Uuid, Callback>>>,
    ) {
        spawn_local(async move {
            while let Some(msg) = ws_read.next().await {
                match msg {
                    Ok(Message::Bytes(b)) => {
                        match bincode::deserialize::<ToFront>(&b[..]) {
                            Ok(val) => {
                                for (_id, cb) in subscribers.borrow().iter() {
                                    (*cb)(&val);
                                }
                            }
                            Err(e) => {
                                log::error!("binary message decode: {:?}", e);
                            }
                        }
                    }
                    Ok(Message::Text(data)) => {
                        log::debug!("text from websocket: {}", data);
                    }
                    Err(e) => {
                        log::error!("websocket message error: {:?}", e)
                    }
                }
            }
            log::debug!("WebSocket Closed");
        });
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
