//! App backend server. Serves the frontend's static files and provides a route
//! for the front<->back websocket connection.
//!
//! Access to the backend is protected by a 64-bit nonce that is sent to the
//! frontend via in-process function calls just before the frontend connects.
//! When the frontend loads the index, a cookie is set that is required for
//! further access to backed routes. This extra layer helps minimize risk of
//! data access by unauthorized programs if the nonce was somehow exposed.
//!
//! The goal here is to prevent other low-privilege user programs from
//! connecting to the backend and retrieving private user data. It does not
//! protect us from programs with sufficient priviledges to sniff the packet
//! activity, but a program with such priviledges has other methods through
//! which it can attack the app's security and user privacy, so we don't try to
//! protect against those threats.

use std::net::TcpListener;
use std::sync::Mutex;

use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{
    config::BrowserSession, storage::CookieSessionStore, SessionMiddleware,
};
use actix_web::dev::Server;
use actix_web::http::header::ContentType;
use actix_web::{
    cookie::{time::Duration, Key},
    http::header::{self, CacheControl, CacheDirective},
    HttpMessage as _, HttpRequest, HttpResponse, Responder,
};
use actix_web::{get, routes, web, App, HttpServer};
use base64::{prelude::BASE64_STANDARD, Engine};
use rand::RngCore;

use crate::app_state::AppState;
use crate::files;
use crate::geojson::{lines_geojson_route, points_geojson_route};
use crate::map::{automap::screen, basemap::map_data_route};
use crate::ws_session::ws_route;

const ONE_DAY: Duration = Duration::days(1);

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
    tokio::spawn(server);

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

/// Track whether the frontend has connected to this UI session already. Guards
/// cookie-based login so it only happens on the first load of the index.
#[derive(Default)]
struct FrontendAuth(pub bool);

fn build(listener: TcpListener, frontend_key: FrontendKey) -> Server {
    // Generate a random secret key to use for signing the auth cookie.
    let secret_key = Key::generate();
    let frontend_auth = web::Data::new(Mutex::new(FrontendAuth::default()));

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
                    .service(web::redirect("/metrics", "./"))
                    .service(web::redirect("/settings", "./"))
                    .service(web::redirect("/settings/{subpath}", "../"))
                    // Cookie-based authentication
                    .app_data(frontend_auth.clone())
                    .wrap(IdentityMiddleware::default())
                    .wrap(
                        SessionMiddleware::builder(
                            CookieSessionStore::default(),
                            secret_key.clone(),
                        )
                        .cookie_name("scoria-auth".to_owned())
                        .cookie_secure(false)
                        .session_lifecycle(
                            BrowserSession::default().state_ttl(ONE_DAY),
                        )
                        .build(),
                    ),
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

/// The index.html route.
///
/// Logs-in the client with a cookie if this is the first connection since
/// server startup or if the cookie is still valid in this session (re-login).
///
/// The content security policy mitigates cross-site scripting by restricting
/// where resources can be loaded from. Inline scripts are blocked, except for
/// the two used in index.html that are authenticated with random nonces.
/// General use of eval is also blocked.
///
/// - 'self' allows loading resources from the same host:post
/// - nonces match those inserted into the inline scripts in index.html
/// - wasm-unsafe-eval required to load wasm
/// - worker-src, child-src, and img-src blobs required by maplibre
/// - style-src required by plotly
#[get("/")]
async fn index(
    req: HttpRequest,
    frontend_auth: web::Data<Mutex<FrontendAuth>>,
    identity: Option<Identity>,
) -> impl Responder {
    if frontend_auth.lock().unwrap().0 {
        // Frontend already logged in, allow re-login only if cookie is valid.
        match identity.map(|id| id.id()) {
            Some(Ok(_)) => (), // valid login from this session, continue
            None | Some(Err(_)) => {
                // no valid login, disallow access
                return HttpResponse::Unauthorized().finish();
            }
        }
    }
    // Set the session cookie required for further requests to sensitive routes.
    Identity::login(&req.extensions(), "user".to_owned()).unwrap();
    frontend_auth.lock().unwrap().0 = true;

    // Generate nonces for the two inline scripts we have, and inject them into
    // the inline script tags.
    let mut rng = rand::thread_rng();
    let mut buf = [0; 32]; // 32 bytes = 256 bits
    let mut nonces = vec![];
    for _ in 0..2 {
        rng.fill_bytes(&mut buf);
        nonces.push(BASE64_STANDARD.encode(buf));
    }
    let new_index = files::get_index()
        .replacen("RANDOM_NONCE", &nonces[0], 1)
        .replacen("RANDOM_NONCE", &nonces[1], 1);
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .insert_header(no_caching_directives())
        .insert_header((
            header::CONTENT_SECURITY_POLICY,
            format!(
                "default-src 'self' ; \
                connect-src 'self' https://scoria.info https://*.scoria.info ; \
                script-src 'self' 'nonce-{}' 'nonce-{}' 'wasm-unsafe-eval' ; \
                worker-src blob: ; \
                child-src blob: ; \
                img-src data: 'self' blob: ; \
                style-src 'self' 'unsafe-inline'",
                nonces[0], nonces[1],
            ),
        ))
        .body(new_index)
}

#[routes]
#[get("/front_wasm_bg.wasm")]
#[get("/settings/front_wasm_bg.wasm")]
async fn wasm(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("content-type", "application/wasm"))
        .body(files::get_wasm())
}

#[routes]
#[get("/front_wasm.js")]
#[get("/settings/front_wasm.js")]
async fn js(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(files::get_js_launcher())
}

#[routes]
#[get("/tailwind.css")]
#[get("/settings/tailwind.css")]
async fn tailwind(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(files::get_tailwind())
}

#[routes]
#[get("/plotly.min.js")]
#[get("/settings/plotly.min.js")]
async fn plotly(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(files::get_plotly())
}

#[routes]
#[get("/maplibre-gl.js")]
#[get("/settings/maplibre-gl.js")]
async fn maplibre(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JAVASCRIPT_UTF_8))
        .body(files::get_maplibre())
}

#[routes]
#[get("/maplibre-gl.css")]
#[get("/settings/maplibre-gl.css")]
async fn maplibre_css(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(files::get_maplibre_css())
}

#[routes]
#[get("/when_in_use_auth.png")]
#[get("/intro/when_in_use_auth.png")]
async fn when_in_use_auth_png(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(files::get_when_in_use_auth())
}

#[routes]
#[get("/always_auth.png")]
#[get("/intro/always_auth.png")]
async fn always_auth_png(_: Identity) -> impl Responder {
    #[cfg(feature = "ios_config")]
    return HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(files::get_always_auth());
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
