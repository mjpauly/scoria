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
    let _result = zip::ZipArchive::new(std::fs::File::open(archive).unwrap())
        .unwrap()
        .extract(destination.clone())
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
