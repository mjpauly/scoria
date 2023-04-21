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
use crate::common::{TimeRange, ToBack, ToFront};
use crate::core::print_and_log;
use crate::database;

/// How often heartbeat pings are sent
#[allow(dead_code)]
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
#[allow(dead_code)]
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our websocket route
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    // Disallow another websocket connection if one is already active
    if AppState::global().ws_addr.lock().unwrap().is_some() {
        print_and_log("Additional UI websocket connection rejected.");
        return Ok(HttpResponse::Unauthorized()
            .body("Only one UI connection allowed."));
    }
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
    #[allow(dead_code)]
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                print_and_log(
                    "Websocket Client heartbeat failed, disconnecting!",
                );

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
        // dbg!(msg.clone());
        match msg {
            ToBack::GetState => {
                self.send_state(ctx);
            }
            ToBack::SetLocationConfig(val) => {
                AppState::global()
                    .persistent
                    .lock()
                    .unwrap()
                    .location_config = val.clone();
                // re-broadcast the new state in case other components are
                // listening for it
                self.send_msg(ToFront::LocationConfig(val), ctx);
            }
            ToBack::GetLocationTimeRange(time_range) => {
                self.send_location_time_range(ctx, time_range);
            }
        }
    }

    /// Encodes a ToFront and sends it over the websocket
    fn send_msg(&self, msg: ToFront, ctx: &mut ws::WebsocketContext<Self>) {
        // dbg!(msg.clone());
        let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
        ctx.binary(encoded);
    }

    /// Sends all UI state values, used at startup.
    fn send_state(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let location_config = AppState::global()
            .persistent
            .lock()
            .unwrap()
            .location_config
            .clone();
        self.send_msg(ToFront::LocationConfig(location_config), ctx);

        // need a future to query the database, so we convert the future
        // into an actor which communicates back to ourselves with the
        // message to send to the frontend (or something like that, see
        // the SO thread linked in the docstring for more)
        let recipient = ctx.address().recipient();
        let fut = async move {
            let rec = database::get_last_record().await;
            if let Some(val) = rec {
                recipient.do_send(MsgToFront(ToFront::LastLocation(val)));
            }
        };
        fut.into_actor(self).spawn(ctx);

        // Send num updates in past hour
        let recipient = ctx.address().recipient();
        let fut = async move {
            // let rec = database::get_last_record().await;
            let count = database::count_records_past_hour().await;
            recipient.do_send(MsgToFront(ToFront::LocationsPastHour(count)));
        };
        fut.into_actor(self).spawn(ctx);
    }

    /// Send location data in a given range of time
    fn send_location_time_range(
        &self,
        ctx: &mut ws::WebsocketContext<Self>,
        time_range: TimeRange,
    ) {
        let recipient = ctx.address().recipient();
        let fut = async move {
            let records = database::get_records_time_range(&time_range).await;
            recipient.do_send(MsgToFront(ToFront::LocationTimeRange(
                time_range, records,
            )));
        };
        fut.into_actor(self).spawn(ctx);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MsgToFront(pub ToFront);

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
        // set the app_state to contain the address of the websocket session
        *AppState::global().ws_addr.lock().unwrap() = Some(ctx.address());

        // Don't need the heartbeat; server shutdown anyways on app
        // backgrounding
        // self.hb(ctx);
    }

    /// Method called on actor stop. Actor is dropped after this function.
    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Unset the actor address in the app state
        *AppState::global().ws_addr.lock().unwrap() = None;
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
                print_and_log(&format!(
                    "Closing Websocket with reason: {:?}",
                    reason
                ));
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
