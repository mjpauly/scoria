//! Connects to the backend via a websocket.
//!
//! Callbacks are ordinary closures which take a &ToFront as an argument
//! and return nothing. They should check the message type using `if let` so
//! they only receive messages they are interested in. If they want to persist
//! the message data, they must clone it.
//!
//! # Using the `use_backend_event` and `use_backend_event_with_deps` hooks
//!
//! ```
//! let data = use_state(|| 0);
//! let on_backend_msg = {
//!     let data = data.clone();
//!     move |msg: &ToFront| {
//!         if let ToFront::Data(val) = msg {
//!             data.set(*val);
//!         }
//!     }
//! };
//! // `on_backend_msg` doesn't consume any state, so it never needs to be
//! // updated. `use_backend_event()` is appropriate:
//! use_backend_event(on_backend_msg);
//! ```
//!
//! ```
//! let counter = use_state(|| 0);
//! let on_backend_msg = {
//!     let counter = counter.clone();
//!     move |msg: &ToFront| {
//!         if let ToFront::Data(val) = msg {
//!             log::debug!( "Callback triggered, value: {}", *counter,);
//!         }
//!     }
//! };
//! // Backend state now consumes the counter state, so we need to use
//! // use_backend_event_with_deps() and pass the counter as the dependency:
//! use_backend_event(on_backend_msg, counter);
//! let onclick = {
//!     let counter = counter.clone();
//!     Callback::from(move |_e: MouseEvent| {
//!         counter.set(*counter + 1);
//!     })
//! };
//! html! {
//!     <button onclick={onclick}>{"increment a counter"}</button>
//! }
//! ```
//!
//! # Deprecated: Using WebsocketService Directly
//!
//! This method doesn't unsubscribe the callback when the parent component is
//! unloaded from the DOM. This is fine for top-level app functions where the
//! parent is never unloaded.
//!
//! Usage example within a Yew function component:
//!
//! ```
//! let wss = use_context::<WebsocketService>().unwrap();
//! let counter = use_state(|| 0);  // warning: always starts at 0
//! let on_backend_msg = {
//!     let counter = counter.clone();
//!     move |msg: &ToFront| {
//!         if let ToFront::Data(val) = msg {
//!             counter.set(*val);
//!         }
//!     }
//! };
//! let id = use_memo(|_| uuid::Uuid::new_v4(), ());
//! wss.subscribe(*id, Box::new(on_backend_msg));
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
//! Based partially on:
//! <https://github.com/jtordgeman/YewChat/blob/websockets-part2/src/services/websocket.rs>

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use futures::channel::mpsc::{Receiver, Sender};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::functional::{hook, use_context, use_effect_with_deps};

// Re-export the message types
pub use crate::common::{ToBack, ToFront};

/// Subscribe to backend events.
///
/// The callback will not be changed upon rerenders. If it should be updated
/// (e.g. it consumes a use_state hook), then use use_backend_event_with_deps()
/// and pass the state hook in as the second argument.
///
/// This hook always maintains a single active callback. When the parent
/// component is unloaded from the DOM, the callback is dropped from tracking by
/// the WebsocketService. When deps change in the case of
/// `use_backend_event_with_deps()`, the old callback is dropped and the new one
/// is saved in its place.
#[hook]
pub fn use_backend_event<F>(callback: F)
where
    F: Fn(&ToFront) + 'static,
{
    // Call use_backend_event_with_deps() with deps set to ():
    use_backend_event_with_deps(callback, ());
}

/// Subscribe to backend events, with updating on rerenders when `deps` change.
///
/// `deps` can be a tuple of states to watch.
///
/// Even for use_state hooks we must watch the handle since the value held will
/// reflect the value at the time the handle is returned by the use_reducer
/// (whatever that means). See the "Caution" section of
/// <https://docs.rs/yew/0.20.0/yew/functional/fn.use_state.html>
#[hook]
pub fn use_backend_event_with_deps<F, T>(callback: F, deps: T)
where
    F: Fn(&ToFront) + 'static, // an ordinary closure
    T: PartialEq + 'static,    // any yew state-compatible structure
{
    // Get the WebsocketService instance from our top-level context
    let wss = use_context::<WebsocketService>().unwrap();
    use_effect_with_deps(
        // Whenever deps change, this closure is called again, which in turn
        // captures the updated callback passed in.
        move |_deps| {
            // Get a unique id for this callback
            let id = uuid::Uuid::new_v4();
            wss.subscribe(id, Box::new(callback));
            // This closure runs on cleanup to unsubscribe the old callback
            move || {
                wss.unsubscribe(id);
            }
        },
        deps,
    );
}

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

    pub fn unsubscribe(&self, id: Uuid) {
        self.subscribers.borrow_mut().remove(&id);
        // log::debug!("removing subscriber {}", id)
    }

    pub fn new() -> Self {
        // Get the port and scope that we connected to on the server
        let location = web_sys::window().unwrap().location();
        let port = location.port().unwrap().parse::<u16>().unwrap();
        let pathname = location.pathname().unwrap();
        let scope = pathname.trim_matches('/').split('/').next().unwrap();
        let address = format!("ws://127.0.0.1:{port}/{scope}/ws");
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
