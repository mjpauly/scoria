/// Actor based implementation of websocket session for the UI
///
/// Based on https://github.com/actix/examples/tree/master/websockets/chat
///
/// Ridiculously helpful SO thread about using async functions with actors:
/// https://stackoverflow.com/questions/64434912/how-to-correctly-call-async-functions-in-a-websocket-handler-in-actix-web
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::time::{Duration, Instant};

use crate::app_state::AppState;
use crate::common::{ToBack, ToFront};
use crate::database::get_last_record;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our websocket route
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(WsSession { hb: Instant::now() }, &req, stream)
}

pub struct WsSession {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,
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

    /// Handles a decoded ToBack
    fn handle_msg(&self, msg: ToBack, ctx: &mut ws::WebsocketContext<Self>) {
        dbg!(msg.clone());
        // TODO
        match msg {
            ToBack::GetState => {
                let location_enabled =
                    *AppState::global().location_is_enabled.lock().unwrap();
                self.send_msg(ToFront::LocationEnabled(location_enabled), ctx);

                // need a future to query the database, so we convert the future
                // into an actor which communicates back to ourselves with the
                // message to send to the frontend (or something like that, see
                // the SO thread linked in the docstring for more)
                let recipient = ctx.address().recipient();
                let fut = async move {
                    let rec = get_last_record().await;
                    if let Some(val) = rec {
                        recipient.do_send(MsgToFront(ToFront::LastLocation(
                            val.into(),
                        )));
                    }
                };
                fut.into_actor(self).spawn(ctx);
            }
            ToBack::SetLocationEnabled(val) => {
                *AppState::global().location_is_enabled.lock().unwrap() = val;
                // re-broadcast the new state in case other components are
                // listening for it
                self.send_msg(ToFront::LocationEnabled(val), ctx);
            }
        }
    }

    /// Encodes a ToFront and sends it over the websocket
    fn send_msg(&self, msg: ToFront, ctx: &mut ws::WebsocketContext<Self>) {
        let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
        ctx.binary(encoded);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct MsgToFront(ToFront);

impl Handler<MsgToFront> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: MsgToFront, ctx: &mut Self::Context) {
        self.send_msg(msg.0, ctx);
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    fn started(&mut self, ctx: &mut Self::Context) {
        // start heartbeat process on session start.
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
        // println!("WEBSOCKET MESSAGE: {msg:?}");
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Binary(bytes) => {
                // deserialize ToBack
                let decoded: ToBack = bincode::deserialize(&bytes[..]).unwrap();
                self.handle_msg(decoded, ctx);
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
