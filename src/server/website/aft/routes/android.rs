//! Routes for info about Android

use actix_web::http::header::ContentType;
use actix_web::{dev::HttpServiceFactory, get, HttpResponse, Responder};

static ANDROID_RELEASE_NOTES_FILE: &str =
    include_str!(env!("ANDROID_RELEASE_NOTES_FILE"));
static ANDROID_NOTES_FILE: &str = include_str!(env!("ANDROID_NOTES_FILE"));

pub fn get_android_services() -> impl HttpServiceFactory {
    (release_notes, android_notes)
}

#[get("/android")]
async fn android_notes() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(ANDROID_NOTES_FILE)
}

#[get("/android/release-notes")]
async fn release_notes() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(ANDROID_RELEASE_NOTES_FILE)
}
