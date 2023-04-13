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

use crate::paths::Paths;
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
pub fn run(
    init_paths: Paths,
    base_url: &str,
    port: u16,
    secure: bool,
) -> ServerConfig {
    unzip_dist(init_paths.clone()).unwrap();

    // If we bind to port 0, the OS assigns us an available port
    let listener = TcpListener::bind(format!("{}:{}", base_url, port)).unwrap();
    let port = listener.local_addr().unwrap().port();
    println!("listening on port {}", port);

    let frontend_key = if secure {
        FrontendKey::new()
    } else {
        FrontendKey::new_insecure()
    };
    println!("frontend key is {}", frontend_key.expose());

    let server = build(init_paths, listener, frontend_key.clone());
    let _server_handle = server.handle(); // TODO: put in AppState
    let _ = tokio::spawn(async move { server.await });
    ServerConfig { port, frontend_key }
}

/// Unzip the frontend components from the bundle into {library_dir}/dist
fn unzip_dist(init_paths: Paths) -> Result<(), String> {
    let archive = init_paths.bundle_dir.join("dist.zip");
    if !archive.exists() {
        return Err("Can't find 'dist.zip' in bundle".to_string());
    }
    let destination = init_paths.library_dir.join("dist");
    zip::ZipArchive::new(std::fs::File::open(archive).unwrap())
        .unwrap()
        .extract(destination)
        .unwrap();
    Ok(())
}

fn build(
    init_paths: Paths,
    listener: TcpListener,
    frontend_key: FrontendKey,
) -> Server {
    let dist = init_paths.library_dir.join("dist"); // static files
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
                    .route("/analyze", web::get().to(index))
                    .route("/test_page", web::get().to(index))
                    // yew-router adds trailing slashes that change the relative
                    // scope that static files are loaded from on reload, so we
                    // redirect those to the routes without the trailing slash
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
}
