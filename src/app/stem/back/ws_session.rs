/// Actor based implementation of websocket session for the UI
///
/// Based on https://github.com/actix/examples/tree/master/websockets/chat
///
/// Ridiculously helpful SO thread about using async functions with actors:
/// https://stackoverflow.com/questions/64434912/how-to-correctly-call-async-functions-in-a-websocket-handler-in-actix-web
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use common::state::{PersistedRoute, PersistedSettingsRoute};
use tracing::{info, warn};

use crate::database;
use crate::geojson::update_geojson;
use crate::logs::update_last_logged_error;
use crate::map::automap::update_automap;
use crate::map::basemap::evict_old_map_data;
use crate::runtime::get_runtime;
use crate::{app_state::AppState, geojson::get_popup_text};
use common::{ToBack, ToFront};

/// How often heartbeat pings are sent
#[allow(dead_code)]
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
#[allow(dead_code)]
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn send_back_state_to_front() {
    send(|addr| addr.do_send(SendState));
}

pub fn send_message_to_front(msg: ToFront) {
    send(|addr| addr.do_send(MsgToFront(msg)));
}

fn send(f: impl FnOnce(actix::Addr<WsSession>)) {
    // We first want to get the address, NOT in the "if let" scrutinee, since
    // the lock will be held for the whole if-block, and we won't be able to
    // await
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    if let Some(addr) = maybe_addr {
        f(addr)
    }
}

/// Entry point for our websocket route
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    // Disallow another websocket connection if one is already active
    if AppState::global().ws_addr.lock().unwrap().is_some() {
        warn!("Additional UI websocket connection rejected.");
        return Ok(HttpResponse::Unauthorized()
            .body("Only one UI connection allowed."));
    }
    info!("Frontend Websocket Connected");
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
                warn!("Websocket Client heartbeat failed, disconnecting!");

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
                let main_route = val.route;
                let settings_route = val.settings_route;
                AppState::global().persistent.lock().unwrap().front =
                    Some(*val);
                AppState::save_to_file();
                get_runtime().spawn(update_geojson(None, false));
                if main_route == PersistedRoute::SettingsSubpage
                    && settings_route == PersistedSettingsRoute::MapSettings
                {
                    // Received state while ui is on the map settings, try
                    // updating the automap in case it was just turned on.
                    get_runtime().spawn(update_automap());
                    // Same goes for evicting data from the map cache (if the
                    // user reduced the cache size).
                    get_runtime().spawn(evict_old_map_data());
                }
                // when the frontend pushes the settings root as the main route,
                // the settings route changes after a moment, so we test both
                if main_route == PersistedRoute::SettingsRoot
                    && settings_route == PersistedSettingsRoute::Root
                {
                    get_runtime().spawn(async move {
                        if let Err(e) = update_last_logged_error().await {
                            tracing::error!(
                                "Failed to update last logged error: {e}"
                            );
                        };
                    });
                }
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
            ToBack::ReviewedLastError => {
                if let Some((_, reviewed)) = &mut AppState::global()
                    .persistent
                    .lock()
                    .unwrap()
                    .back
                    .last_logged_error
                {
                    *reviewed = true;
                }
            }
            ToBack::GetPopupText((location, data_color)) => {
                let recipient = ctx.address().recipient();
                get_runtime().spawn(async move {
                    if let Some(msg) =
                        get_popup_text(location, data_color).await
                    {
                        recipient.do_send(MsgToFront(msg))
                    }
                });
            }
        }
    }

    /// Encodes a ToFront and sends it over the websocket
    fn send_msg(&self, msg: ToFront, ctx: &mut ws::WebsocketContext<Self>) {
        // dbg!(msg.clone());
        // unwrap here since something this core to the app's function should
        // just crash it
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
        get_runtime().spawn(async move {
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
        });
    }

    /// Sends all UI state values, used at startup.
    fn send_front_state(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let recipient = ctx.address().recipient();
        get_runtime().spawn(async move {
            // clone the state and send it
            let front =
                AppState::global().persistent.lock().unwrap().front.clone();
            recipient.do_send(MsgToFront(ToFront::FrontState(front)));
        });
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
                warn!("Closing websocket with reason: {reason:?}");
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
