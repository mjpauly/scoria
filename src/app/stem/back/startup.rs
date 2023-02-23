use actix_files as fs;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web::{HttpResponse, Responder};

use crate::paths;

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

pub fn run(base_url: &str, port: u16) {
    unzip_dist();
    let server = build(base_url, port);
    let _ = tokio::spawn(async move { server.await });
}

fn build(base_url: &str, port: u16) -> Server {
    let dist = paths::get_library_dir().unwrap().join("dist"); // static files
    HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .service(fs::Files::new("/", dist.clone()).index_file("index.html"))
    })
    .bind(format!("{}:{}", base_url, port))
    .unwrap()
    .run()
}

/// Unzip the frontend components from the bundle into {library_dir}/dist
fn unzip_dist() {
    let archive = paths::get_bundle_dir().unwrap().join("dist.zip");
    if !archive.exists() {
        panic!("Can't find 'dist.zip' in bundle");
    }
    let destination = paths::get_library_dir().unwrap();
    let _result = zip::ZipArchive::new(std::fs::File::open(archive).unwrap())
        .unwrap()
        .extract(destination.clone())
        .unwrap();
}
