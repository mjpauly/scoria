use std::net::TcpListener;

use actix_files as fs;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web::{HttpResponse, Responder};

use crate::paths::Paths;
use crate::ws_session::ws_route;

/// Start the server backend in a new tokio task, returning a handle to the
/// server
pub fn run(init_paths: Paths, base_url: &str, port: u16) -> u16 {
    unzip_dist(init_paths.clone()).unwrap();

    // If we bind to port 0, the OS assigns us an available port
    let listener = TcpListener::bind(format!("{}:{}", base_url, port)).unwrap();
    let port = listener.local_addr().unwrap().port();
    println!("listening on port {}", port);

    let server = build(init_paths, listener);
    let _server_handle = server.handle(); // TODO: put in AppState
    let _ = tokio::spawn(async move { server.await });
    port
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

fn build(init_paths: Paths, listener: TcpListener) -> Server {
    let dist = init_paths.library_dir.join("dist"); // static files
    HttpServer::new(move || {
        let files_service =
            fs::Files::new("/", dist.clone()).index_file("index.html");
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/ws", web::get().to(ws_route))
            .service(files_service)
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run()
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
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
            .get(&format!("http://127.0.0.1:{}/health_check", port))
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert!(response.status().is_success());
        assert_eq!(Some(0), response.content_length());
    }
}
