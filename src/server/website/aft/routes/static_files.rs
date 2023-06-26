use actix_web::http::header::ContentType;
use actix_web::{get, HttpResponse, Responder};

static INDEX_FILE: &str = include_str!(env!("INDEX_FILE"));
static PRIVACY_POLICY_FILE: &str = include_str!(env!("PRIVACY_POLICY_FILE"));
static CONTACT_FILE: &str = include_str!(env!("CONTACT_FILE"));
static TAILWIND_FILE: &str = include_str!(env!("TAILWIND_FILE"));
static LOGO_FILE: &[u8] = include_bytes!(env!("LOGO_FILE"));
static FAVICON_FILE: &[u8] = include_bytes!(env!("FAVICON_FILE"));
pub static TEMPLATE_TOP_FILE: &str = include_str!(env!("TEMPLATE_TOP_FILE"));
pub static TEMPLATE_BOTTOM_FILE: &str =
    include_str!(env!("TEMPLATE_BOTTOM_FILE"));

pub fn get_static_file_services(
) -> (index, privacy_policy, contact, tailwind, logo, favicon) {
    (index, privacy_policy, contact, tailwind, logo, favicon)
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(INDEX_FILE)
}

#[get("/privacy")]
async fn privacy_policy() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(PRIVACY_POLICY_FILE)
}

#[get("/contact")]
async fn contact() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(CONTACT_FILE)
}

#[get("/tailwind.css")]
async fn tailwind() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(TAILWIND_FILE)
}

#[get("/epsilon.png")]
async fn logo() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(LOGO_FILE)
}

#[get("/favicon.png")]
async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_PNG))
        .body(FAVICON_FILE)
}
