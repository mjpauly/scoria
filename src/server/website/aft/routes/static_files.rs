use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse, Responder};

static INDEX_FILE: &str = include_str!(env!("INDEX_FILE"));
static FIRST_RELEASE_POST: &str = include_str!(env!("FIRST_RELEASE_POST"));
static PRIVACY_POLICY_FILE: &str = include_str!(env!("PRIVACY_POLICY_FILE"));
static CONTACT_FILE: &str = include_str!(env!("CONTACT_FILE"));
static TERMS_FILE: &str = include_str!(env!("TERMS_FILE"));
static TAILWIND_FILE: &str = include_str!(env!("TAILWIND_FILE"));
static LOGO_FILE: &[u8] = include_bytes!(env!("LOGO_FILE"));
static FAVICON_FILE: &[u8] = include_bytes!(env!("FAVICON_FILE"));
pub static TEMPLATE_TOP_FILE: &str = include_str!(env!("TEMPLATE_TOP_FILE"));
pub static TEMPLATE_BOTTOM_FILE: &str =
    include_str!(env!("TEMPLATE_BOTTOM_FILE"));
static SCREENSHOT1: &[u8] = include_bytes!(env!("SCREENSHOT1"));
static SCREENSHOT2: &[u8] = include_bytes!(env!("SCREENSHOT2"));
static SCREENSHOT3: &[u8] = include_bytes!(env!("SCREENSHOT3"));
static SCREENSHOT4: &[u8] = include_bytes!(env!("SCREENSHOT4"));
static APP_STORE_BADGE: &[u8] = include_bytes!(env!("APP_STORE_BADGE"));

pub fn get_static_file_services() -> (
    index,
    first_release_post,
    privacy_policy,
    contact,
    terms,
    tailwind,
    logo,
    favicon,
    screenshots,
    app_store_badge,
) {
    (
        index,
        first_release_post,
        privacy_policy,
        contact,
        terms,
        tailwind,
        logo,
        favicon,
        screenshots,
        app_store_badge,
    )
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(INDEX_FILE)
}

#[get("/posts")]
async fn first_release_post() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(FIRST_RELEASE_POST)
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

#[get("/terms")]
async fn terms() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(TERMS_FILE)
}

#[get("/tailwind.css")]
async fn tailwind() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS_UTF_8))
        .body(TAILWIND_FILE)
}

#[get("/scoria.png")]
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

#[get("/screenshots/{num}.jpeg")]
async fn screenshots(path: web::Path<usize>) -> impl Responder {
    let bytes = match path.into_inner() {
        1 => SCREENSHOT1,
        2 => SCREENSHOT2,
        3 => SCREENSHOT3,
        4 => SCREENSHOT4,
        _ => return HttpResponse::NotFound().finish(),
    };
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_JPEG))
        .body(bytes)
}

#[get("/app_store_badge.svg")]
async fn app_store_badge() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::IMAGE_SVG))
        .body(APP_STORE_BADGE)
}
