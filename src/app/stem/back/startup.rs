use actix_files as fs;
use actix_web::dev::{Server, ServerHandle};
use actix_web::{web, App, HttpServer};
use actix_web::{HttpResponse, Responder};

use crate::paths;
use crate::{app_state, app_state::AppState};

/// Start the server backend in a new tokio task, returning a handle to the
/// server
//TODO: refactor so this module keeps the handle itself
pub fn run(base_url: &str, port: u16) -> Result<ServerHandle, String> {
    if let Err(e) = unzip_dist() {
        eprintln!("{}", e);
        return Err("Could not unzip dist".to_string());
    }
    let state = app_state::get_app_state();
    let server = build(base_url, port, state);
    let server_handle = server.handle();
    let _ = tokio::spawn(async move { server.await });
    Ok(server_handle)
}

/// Unzip the frontend components from the bundle into {library_dir}/dist
fn unzip_dist() -> Result<(), String> {
    let archive = paths::get_bundle_dir().unwrap().join("dist.zip");
    if !archive.exists() {
        return Err("Can't find 'dist.zip' in bundle".to_string());
    }
    let destination = paths::get_library_dir().unwrap().join("dist");
    let _result = zip::ZipArchive::new(std::fs::File::open(archive).unwrap())
        .unwrap()
        .extract(destination.clone())
        .unwrap();
    Ok(())
}

fn build(base_url: &str, port: u16, state: AppState) -> Server {
    let dist = paths::get_library_dir().unwrap().join("dist"); // static files
    let state = web::Data::new(state);
    HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .service(fs::Files::new("/", dist.clone()).index_file("index.html"))
            .app_data(state.clone())
    })
    .bind(format!("{}:{}", base_url, port))
    .unwrap()
    .run()
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
