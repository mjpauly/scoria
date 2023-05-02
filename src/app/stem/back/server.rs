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

use rand::RngCore;

use actix_files::{Files, NamedFile};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web::{HttpResponse, Responder};

use crate::app_state::AppState;
// use crate::core::print_and_log;
use crate::paths;
use crate::ws_session::ws_route;

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
    // print_and_log(&format!("listening on port {}", port));

    let frontend_key = if secure {
        FrontendKey::new()
    } else {
        FrontendKey::new_insecure()
    };
    // print_and_log(&format!("frontend key is {}", frontend_key.expose()));

    let server = build(listener, frontend_key.clone());
    let server_handle = server.handle();
    let _ = tokio::spawn(async move { server.await });

    // Save the server handle so we can stop it later on
    *AppState::global().server_handle.lock().await = Some(server_handle);
    ServerConfig { port, frontend_key }
}

/// Shut down the server (called when the app goes to the background)
pub async fn shutdown() {
    // print_and_log("Shutting down server");
    AppState::global()
        .server_handle
        .lock()
        .await
        .as_mut()
        .unwrap()
        .stop(false) // false: not graceful
        .await;
    // Drop the server handle
    *AppState::global().server_handle.lock().await = None;
}

/// Unzip the frontend components from the bundle into {library_dir}/dist.
/// Must be run during app initialization before the server first starts up.
pub fn unzip_dist() {
    let archive = paths::get_bundle_dir().join("dist.zip");
    if !archive.exists() {
        panic!("Can't find 'dist.zip' in bundle");
    }
    let destination = paths::get_library_dir().join("dist");
    zip::ZipArchive::new(std::fs::File::open(archive).unwrap())
        .unwrap()
        .extract(destination)
        .unwrap();
}

fn build(listener: TcpListener, frontend_key: FrontendKey) -> Server {
    let dist = paths::get_library_dir().join("dist"); // static files
    let index_file = dist.join("index.html");
    HttpServer::new(move || {
        let files_service =
            Files::new("/", dist.clone()).index_file("index.html");
        let index_file = web::Data::new(index_file.clone());
        let scope = format!("{}", frontend_key.clone().expose());
        App::new()
            // redirect scope so that static files are properly loaded from the
            // correct relative path even if a trailing slash is not provided
            .service(web::redirect(format!("/{scope}"), format!("/{scope}/")))
            .service(
                web::scope(&scope)
                    .route("/health_check", web::get().to(health_check))
                    .route("/ws", web::get().to(ws_route))
                    // extra SPA routes we want to just get the index file for
                    .route("/sense", web::get().to(index))
                    .route("/analyze", web::get().to(index))
                    .route("/test_page", web::get().to(index))
                    // yew-router adds trailing slashes that change the relative
                    // scope that static files are loaded from on reload, so we
                    // redirect those to the routes without the trailing slash
                    .service(web::redirect("/sense/", "../sense"))
                    .service(web::redirect("/analyze/", "../analyze"))
                    .service(web::redirect("/test_page/", "../test_page"))
                    // static files service includes index file at root
                    .service(files_service)
                    .app_data(index_file),
            )
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run()
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

async fn index(index_file: web::Data<std::path::PathBuf>) -> impl Responder {
    NamedFile::open_async(index_file.get_ref()).await
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
        init(paths).await;
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
