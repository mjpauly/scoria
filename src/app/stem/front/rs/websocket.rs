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
//! let counter = use_state(|| 0);
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
//! The implementation is based on Rc<RefCell<>> so it is not thread safe.
//!
//! Based on:
//! https://github.com/jtordgeman/YewChat/blob/websockets-part2/src/services/websocket.rs

use futures::channel::mpsc::{Receiver, Sender};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::cell::RefCell;
use std::rc::Rc;
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
    subscribers: Rc<RefCell<Vec<Callback>>>,
}

impl WebsocketService {
    /// Send a message to the websocket
    pub fn send_msg(&self, msg: ToBack) {
        self.tx.borrow_mut().try_send(msg).unwrap();
    }

    /// Subscribe to messages from the backend
    pub fn subscribe(&self, cb: Callback) {
        self.subscribers.borrow_mut().push(cb);
    }

    pub fn new() -> Self {
        let ws = WebSocket::open("ws://127.0.0.1:8081/ws").unwrap();

        let (ws_write, ws_read) = ws.split();

        // components write yew_tx, and WebsocketService receives on rx
        let (yew_tx, yew_rx) = futures::channel::mpsc::channel::<ToBack>(1000);

        let subscribers = Rc::new(RefCell::new(Vec::<Callback>::new()));

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
        subscribers: Rc<RefCell<Vec<Callback>>>,
    ) {
        spawn_local(async move {
            while let Some(msg) = ws_read.next().await {
                match msg {
                    Ok(Message::Bytes(b)) => {
                        match bincode::deserialize::<ToFront>(&b[..]) {
                            Ok(val) => {
                                for sub in subscribers.borrow().iter() {
                                    (*sub)(&val);
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
