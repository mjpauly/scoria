use actix_web::http::header::ContentType;
use actix_web::routes;
use actix_web::{dev::HttpServiceFactory, get, web, HttpResponse, Responder};

use crate::configuration::TurnstileSettings;

static INDEX_FILE: &str = include_str!(env!("INDEX_FILE"));
static FEED_FILE: &str = include_str!(env!("FEED_FILE"));
static POSTS_FILE: &str = include_str!(env!("POSTS_FILE"));
static TIPS_FOR_USE_POST: &str = include_str!(env!("TIPS_FOR_USE_POST"));
static IMPORT_PLACES_POST: &str = include_str!(env!("IMPORT_PLACES_POST"));
static SCORIA_1_4_0_POST: &str = include_str!(env!("1_4_0_RELEASE_POST"));
static ANDROID_RELEASE_POST: &str = include_str!(env!("ANDROID_RELEASE_POST"));
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
static SCREENSHOT5: &[u8] = include_bytes!(env!("SCREENSHOT5"));
static SCREENSHOT6: &[u8] = include_bytes!(env!("SCREENSHOT6"));
static SCREENSHOT7: &[u8] = include_bytes!(env!("SCREENSHOT7"));
static APP_STORE_BADGE: &[u8] = include_bytes!(env!("APP_STORE_BADGE"));

/// Keeps well-behaved crawlers from downloading every APK on each visit.
static ROBOTS_FILE: &str = "User-agent: *\nDisallow: /download/apk/\n";

/// Max 12 elements can be grouped together
pub fn get_static_file_services() -> impl HttpServiceFactory {
    (
        index,
        posts_services(),
        privacy_policy,
        contact,
        terms,
        tailwind,
        logo,
        favicon,
        screenshots,
        app_store_badge,
        robots,
    )
}

fn posts_services() -> impl HttpServiceFactory {
    (
        feed,
        posts,
        tips_for_use_post,
        import_places_post,
        scoria_1_4_0_post,
        android_release_post,
        first_release_post,
    )
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(INDEX_FILE)
}

#[routes]
#[get("/feed")]
#[get("/feed/")]
async fn feed() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("content-type", "application/atom+xml"))
        .body(FEED_FILE)
}

#[get("/posts")]
async fn posts() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(POSTS_FILE)
}

#[get("/posts/tips_for_use")]
async fn tips_for_use_post() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(TIPS_FOR_USE_POST)
}

#[get("/posts/import_places")]
async fn import_places_post() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(IMPORT_PLACES_POST)
}

#[get("/posts/Scoria_1.4.0")]
async fn scoria_1_4_0_post() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(SCORIA_1_4_0_POST)
}

#[get("/posts/android_release")]
async fn android_release_post() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(ANDROID_RELEASE_POST)
}

#[get("/posts/first_release")]
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
async fn contact(turnstile: web::Data<TurnstileSettings>) -> impl Responder {
    HttpResponse::Ok().content_type(ContentType::html()).body(
        CONTACT_FILE.replace("{{TURNSTILE_SITE_KEY}}", &turnstile.site_key),
    )
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
        5 => SCREENSHOT5,
        6 => SCREENSHOT6,
        7 => SCREENSHOT7,
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

#[get("/robots.txt")]
async fn robots() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(ROBOTS_FILE)
}
