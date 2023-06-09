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

use crate::core::print_and_log;
use crate::database;
use crate::geojson::update_geojson;
use crate::{app_state::AppState, geojson::get_popup_text};
use common::{ToBack, ToFront};

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
            ToBack::GetFrontState => {
                self.send_front_state(ctx);
            }
            ToBack::GetBackState => {
                self.send_back_state(ctx);
            }
            ToBack::SetFrontState(val) => {
                AppState::global().persistent.lock().unwrap().front = Some(val);
                AppState::save_to_file();
                let fut = async move {
                    update_geojson(None, false).await;
                };
                fut.into_actor(self).spawn(ctx);
            }
            ToBack::ExportSqliteLog => {
                AppState::global()
                    .swift_messages
                    .lock()
                    .unwrap()
                    .should_export_sqlite_log = true;
            }
            ToBack::ImportSqliteLog => {
                AppState::global()
                    .swift_messages
                    .lock()
                    .unwrap()
                    .should_import_sqlite_log = true;
            }
            ToBack::RequestWhenInUseAuthorization => {
                AppState::global()
                    .swift_messages
                    .lock()
                    .unwrap()
                    .should_request_when_in_use_authorization = true;
            }
            ToBack::GetPopupText((location, data_color)) => {
                let recipient = ctx.address().recipient();
                let fut = async move {
                    recipient.do_send(MsgToFront(
                        get_popup_text(location, data_color).await,
                    ))
                };
                fut.into_actor(self).spawn(ctx);
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
    fn send_back_state(&self, ctx: &mut ws::WebsocketContext<Self>) {
        // need a future to query the database, so we convert the future
        // into an actor which communicates back to ourselves with the
        // message to send to the frontend (or something like that, see
        // the SO thread linked in the docstring for more)
        let recipient = ctx.address().recipient();
        let fut = async move {
            // update the last location and number of records in the past hour
            let rec = database::get_last_record().await;
            AppState::global()
                .persistent
                .lock()
                .unwrap()
                .back
                .last_location = rec;
            let n = database::count_records_past_hour().await;
            AppState::global()
                .persistent
                .lock()
                .unwrap()
                .back
                .locations_past_hour = Some(n);
            // clone the state and send it
            let back =
                AppState::global().persistent.lock().unwrap().back.clone();
            recipient.do_send(MsgToFront(ToFront::BackState(back)));
        };
        fut.into_actor(self).spawn(ctx);
    }

    /// Sends all UI state values, used at startup.
    fn send_front_state(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let recipient = ctx.address().recipient();
        let fut = async move {
            // clone the state and send it
            let front =
                AppState::global().persistent.lock().unwrap().front.clone();
            recipient.do_send(MsgToFront(ToFront::FrontState(front)));
        };
        fut.into_actor(self).spawn(ctx);
    }
}

/// Send a particular message to the frontend.
#[derive(Message)]
#[rtype(result = "()")]
pub struct MsgToFront(pub ToFront);

impl Handler<MsgToFront> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: MsgToFront, ctx: &mut Self::Context) {
        self.send_msg(msg.0, ctx);
    }
}

/// Message to indicate that a new BackState should be sent to the frontend
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendState;

impl Handler<SendState> for WsSession {
    type Result = ();

    fn handle(&mut self, _msg: SendState, ctx: &mut Self::Context) {
        self.send_back_state(ctx);
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
