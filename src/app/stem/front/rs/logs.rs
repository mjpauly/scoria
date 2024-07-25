//! Logging to the javascript console and to the backend.

use std::fmt::Write;
use std::sync::{Arc, Mutex};

use futures::channel::mpsc::Sender;
use tracing::{
    field::{Field, Visit},
    Level,
};
use tracing_subscriber::reload;
use tracing_subscriber::{prelude::*, EnvFilter, Layer, Registry};
use tracing_web::MakeWebConsoleWriter;

use common::ToBack;

/// The handle to reload the websocket log listener layer.
type Handle = reload::Handle<WebsocketLogLayer, Registry>;

/// Initialize logging, before starting up the websocket.
///
/// Returns a handle to reload the websocket log listener once the websocket
/// is running. This ordering ensure we can debug the websocket during
/// development using tracing, instead of having to wait to initialize the
/// subscriber until afterwards.
pub fn init_logging() -> Handle {
    let env_filter = EnvFilter::new("error,front=debug");

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_ansi(false) // Only partially supported across browsers
        .without_time() // std::time is not available in browsers
        .with_writer(MakeWebConsoleWriter::new().with_pretty_level())
        .with_level(false);

    let (custom_layer, reload_handle) =
        reload::Layer::<_, _>::new(WebsocketLogLayer(None));

    tracing_subscriber::registry()
        .with(custom_layer)
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!("Initialized logs");

    reload_handle
}

pub fn reload_with_ws_log_listener(handle: Handle, sender: Sender<ToBack>) {
    handle
        .modify(|layer| {
            *layer = WebsocketLogLayer(Some(Arc::new(Mutex::new(sender))))
        })
        .unwrap();
}

/// The log layer for sending errors to be persisted in the backend.
///
/// Initially the websocket is not started and this Layer does nothing.
/// After logging has started, it is reloaded with a handle to the mpsc
/// channel to send messages to the backend via the websocket service.
///
/// We need the Arc<Mutex<>> to provide thread-safe mutable access to the
/// Sender from an immutable handle to WebsocketLogLayer in the on_event()
/// call.
#[derive(Clone)]
pub struct WebsocketLogLayer(Option<Arc<Mutex<Sender<ToBack>>>>);

impl<S> Layer<S> for WebsocketLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(sender) = &self.0 {
            if event.metadata().level() <= &Level::ERROR {
                let mut error_string = format!(
                    "{}:",
                    event
                        .metadata()
                        .module_path()
                        .unwrap_or(event.metadata().target()),
                );
                if let Some(line) = event.metadata().line() {
                    write!(error_string, " {}:", line).unwrap();
                }
                let mut visitor = StringVisitor::default();
                event.record(&mut visitor);
                error_string.push_str(&visitor.0);
                sender
                    .lock()
                    .unwrap()
                    .try_send(ToBack::LogError(error_string))
                    .unwrap();
            }
        }
    }
}

#[derive(Default)]
struct StringVisitor(String);

impl Visit for StringVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            write!(self.0, " {:?}", value).unwrap();
        } else {
            write!(self.0, " {}={:?}", field.name(), value).unwrap();
        }
    }
}
