//! App backend server. Serves the frontend's static files and provides a route
//! for the front<->back websocket connection.
//!
//! Access to the backend is protected by hiding it behind a key that
//! is sent to the frontend via a private side channel. The goal here
//! is to prevent other low-privilege user programs from connecting
//! to the backend and retrieving private user data. It does not
//! protect us from programs with sufficient priviledges to sniff the
//! packet activity, but a program with such priviledges has other
//! methods through which it can attack the app's security and user
//! privacy, so we don't try to protect against those threats.

use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::http::header::ContentType;
use actix_web::{get, routes, web, App, HttpServer};
use actix_web::{
    http::header::{CacheControl, CacheDirective},
    HttpResponse, Responder,
};
use rand::RngCore;

use crate::app_state::AppState;
use crate::geojson::{lines_geojson_route, points_geojson_route};
use crate::map::{automap::screen, basemap::map_data_route};
use crate::ws_session::ws_route;

#[cfg(not(any(feature = "ios_config", feature = "android_config")))]
compile_error!(
    "Either feature \"ios_config\" or \"android_config\" must be enabled."
);

// static files to serve (env vars are set by bazel and poin to file path)
static INDEX_FILE: &str = include_str!(env!("INDEX_FILE"));
static WASM_FILE: &[u8] = include_bytes!(env!("WASM_FILE"));
static JS_FILE: &str = include_str!(env!("JS_FILE"));
static TAILWIND_FILE: &str = include_str!(env!("TAILWIND_FILE"));
static PLOTLY_FILE: &str = include_str!(env!("PLOTLY_FILE"));
static MAPLIBRE_FILE: &str = include_str!(env!("MAPLIBRE_FILE"));
static MAPLIBRE_CSS: &str = include_str!(env!("MAPLIBRE_CSS"));

#[cfg(feature = "ios_config")]
static WHEN_IN_USE_AUTH_PNG: &[u8] =
    include_bytes!(env!("WHEN_IN_USE_AUTH_IOS_PNG"));
#[cfg(feature = "android_config")]
static WHEN_IN_USE_AUTH_PNG: &[u8] =
    include_bytes!(env!("WHEN_IN_USE_AUTH_ANDROID_PNG"));

#[cfg(feature = "ios_config")]
static ALWAYS_AUTH_PNG: &[u8] = include_bytes!(env!("ALWAYS_AUTH_PNG"));

/// Configuration struct we pass to Swift via C
#[repr(C)]
pub struct ServerConfig {
    pub port: u16,
    pub frontend_key: FrontendKey,
}

/// Key type that ensures we correctly seed the key with entropy on init
///
/// Also referred to as the "scope," since it's used as a web scope behind
/// which the backend server functionality is hidden.
#[repr(C)]
#[derive(Clone)]
pub struct FrontendKey(u64);

#[allow(clippy::new_without_default)]
impl FrontendKey {
    pub fn new() -> Self {
        Self(rand::thread_rng().next_u64())
    }
    pub fn new_insecure() -> Self {
        Self(123)
    }
    pub fn expose(&self) -> u64 {
        self.0
    }
}

/// Start the server backend in a new tokio task, returning the port and
/// frontend key to the caller.
///
/// Secure determines if the key should be randomly generated or set to a small
/// known value (123) for local testing.
pub async fn run(port: u16, secure: bool) -> ServerConfig {
    // If we bind to port 0, the OS assigns us an available port
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    let port = listener.local_addr().unwrap().port();

    let frontend_key = if secure {
        FrontendKey::new()
    } else {
        FrontendKey::new_insecure()
    };

    let server = build(listener, frontend_key.clone());
    let server_handle = server.handle();
    let _ = tokio::spawn(async move { server.await });

    // Save the server handle so we can stop it later on
    *AppState::global().server_handle.lock().await = Some(server_handle);
    ServerConfig { port, frontend_key }
}

/// Shut down the server (called when the app goes to the background)
pub async fn shutdown() {
    if let Some(handle) = AppState::global().server_handle.lock().await.as_mut()
    {
        // false: not graceful
        handle.stop(false).await;
    }
    // Drop the server handle
    *AppState::global().server_handle.lock().await = None;
}

fn build(listener: TcpListener, frontend_key: FrontendKey) -> Server {
    HttpServer::new(move || {
        let scope = format!("{}", frontend_key.clone().expose());
        App::new()
            // redirect scope so that static files are properly loaded from the
            // correct relative path even if a trailing slash is not provided
            .service(web::redirect(format!("/{scope}"), format!("/{scope}/")))
            .service(
                web::scope(&scope)
                    // static files
                    .service(index)
                    .service(wasm)
                    .service(js)
                    .service(tailwind)
                    .service(plotly)
                    .service(maplibre)
                    .service(maplibre_css)
                    .service(when_in_use_auth_png)
                    .service(always_auth_png)
                    // dynamic routes
                    .service(health_check)
                    .service(points_geojson_route)
                    .service(lines_geojson_route)
                    .service(map_data_route)
                    .service(screen)
                    .route("/ws", web::get().to(ws_route))
                    // If the UI crashes, the browser may reload it at the same
                    // path, so we want to redirect that back to the index
                    .service(web::redirect("/analyze", "./"))
                    .service(web::redirect("/intro", "./"))
                    .service(web::redirect("/sense", "./"))
                    .service(web::redirect("/settings", "./"))
                    .service(web::redirect("/settings/{subpath}", "../")),
            )
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run()
}

/// Directives for responses to limit the extent that the browser caches them,
/// since we're already doing that outselves for map data and location data.
///
/// - private: don't put data in a shared cache (viewing region can leak info)
/// - no-store: don't store cache on disk, only memory
/// - must-revalidate: disallow using stale responses, which should prompt
///     the browser to delete them
pub fn no_caching_directives() -> CacheControl {
    CacheControl(vec![
        CacheDirective::NoStore,
        CacheDirective::MustRevalidate,
        CacheDirective::Private,
    ])
}

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .insert_header(no_caching_directives())
        .body(INDEX_FILE)
}

#[routes]
#[get("/front_wasm_bg.wasm")]
#[get("/settings/front_wasm_bg.wasm")]
async fn wasm() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("content-type", "application/wasm"))
        .body(WASM_FILE)
}

#[routes]
#[get("/front_wasm.js")]
#[get("/settings/front_wasm.js")]
async fn js() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(JS_FILE)
}

#[routes]
#[get("/tailwind.css")]
#[get("/settings/tailwind.css")]
async fn tailwind() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(TAILWIND_FILE)
}

#[routes]
#[get("/plotly.min.js")]
#[get("/settings/plotly.min.js")]
async fn plotly() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(PLOTLY_FILE)
}

#[routes]
#[get("/maplibre-gl.js")]
#[get("/settings/maplibre-gl.js")]
async fn maplibre() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(MAPLIBRE_FILE)
}

#[routes]
#[get("/maplibre-gl.css")]
#[get("/settings/maplibre-gl.css")]
async fn maplibre_css() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(MAPLIBRE_CSS)
}

#[routes]
#[get("/when_in_use_auth.png")]
#[get("/intro/when_in_use_auth.png")]
async fn when_in_use_auth_png() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(WHEN_IN_USE_AUTH_PNG)
}

#[routes]
#[get("/always_auth.png")]
#[get("/intro/always_auth.png")]
async fn always_auth_png() -> impl Responder {
    #[cfg(feature = "ios_config")]
    return HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(ALWAYS_AUTH_PNG);
    #[cfg(feature = "android_config")]
    return HttpResponse::NotFound().finish();
}

#[cfg(test)]
mod tests {
    use crate::local::test_setup;

    /// Test that the health check works. This can be run as a unit tests only
    /// because it doesn't touch the global app state. If we did try to connect
    /// to the websocket, then we'd get an error that we haven't initialized the
    /// app-state yet, since we're in a different thread from the one that did
    /// the initialization.
    #[tokio::test]
    async fn server_health_check_works() {
        // Arrange
        let port = test_setup("server_health_check/").await;

        let client = reqwest::Client::new();

        // Act
        let response = client
            .get(&format!("http://127.0.0.1:{}/123/health_check", port))
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert!(response.status().is_success());
        assert_eq!(Some(0), response.content_length());
    }

    use super::{run, shutdown};
    use crate::init;
    use crate::local::local_fs_setup;

    /// Test that after shutting down the server it is not possible to connect
    /// to it anymore
    #[tokio::test]
    async fn server_shutdown_works() {
        let dir = "server_shutdown_works/";
        let paths = local_fs_setup(dir);
        init(paths, "1.test.0".into()).await;
        let cfg = run(0, true).await;

        let url = format!(
            "http://127.0.0.1:{}/{}/health_check",
            cfg.port,
            cfg.frontend_key.expose()
        );

        let client = reqwest::Client::new();

        // Check that we can get a successful connection first
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(response.status().as_u16(), 200);

        // Showdown the server and check that we get a connection failure
        shutdown().await;

        let result = client.get(&url).send().await;
        assert!(result.is_err());
    }
}
