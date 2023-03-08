//! Connects to the backend via a websocket.
//!
//! Based on:
//! https://github.com/jtordgeman/YewChat/blob/websockets-part2/src/services/websocket.rs

use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::cell::RefCell;
use wasm_bindgen_futures::spawn_local;

use crate::{event_bus, event_bus::Request};

thread_local!(static WSS: RefCell<WebsocketService> =
              RefCell::new(WebsocketService::new()));

pub fn send_msg(msg: String) {
    WSS.with(|w| {
        (*w.borrow_mut()).tx.try_send(msg).unwrap();
    });
}

struct WebsocketService {
    tx: Sender<String>,
}

impl WebsocketService {
    fn new() -> Self {
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

        spawn_local(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        log::debug!("from websocket: {}", data);
                        event_bus::send(Request::EventBusMsg(data));
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

        Self { tx: in_tx }
    }
}
