//! The state of the app's configuration

use std::sync::{Arc, Mutex};
// because we are using a standard Mutex, we cannot hold it across .await points

pub type AppState = Arc<AppStateContents>;

#[derive(Debug)]
pub struct AppStateContents {
    pub location_is_enabled: Mutex<bool>,
    pub distance_filter: Mutex<f32>,
}

pub trait AppStateExtentions {
    fn init_state() -> Arc<AppStateContents>;
}

impl AppStateExtentions for AppState {
    /// AppStateContents is wrapped in an Arc so it can be passed between
    /// threads. Actix already does this for web::Data<>, but we need to share
    /// it with code outside the webserver, so we put up with the overhead of
    /// having two Arcs.
    ///
    /// See https://actix.rs/docs/application/#state for more info.
    fn init_state() -> Arc<AppStateContents> {
        Arc::new(AppStateContents {
            // TODO: read from file or do default
            location_is_enabled: Mutex::new(false),
            distance_filter: Mutex::new(5.0),
        })
    }
}

/// Actor based implementation of websocket state for the UI
///
/// Based on https://github.com/actix/examples/tree/master/websockets/chat

#[path = "../front/rs/common.rs"]
mod common;

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::{Duration, Instant};

use common::{MsgForBackend, MsgForFrontend};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our websocket route
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsSession {
            count: 0,
            hb: Instant::now(),
        },
        &req,
        stream,
    )
}

pub struct WsSession {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,

    pub count: usize, // test state
}

impl WsSession {
    /// helper method that sends ping to client every 5 seconds
    /// (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    fn started(&mut self, ctx: &mut Self::Context) {
        // TODO: pull state from file or default
        // self.count = 0;

        // we'll start heartbeat process on session start.
        self.hb(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };
        println!("WEBSOCKET MESSAGE: {msg:?}");
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
                ctx.text("text from WsSession");
            }
            ws::Message::Binary(bytes) => {
                // deserialize MsgForBackend
                let cur = Cursor::new(&bytes[..]);
                let mut de = Deserializer::new(cur); // msgpack deserializer
                let msg: MsgForBackend =
                    Deserialize::deserialize(&mut de).unwrap();
                dbg!(msg);
            }
            ws::Message::Text(text) => println!("got text {}", text),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
