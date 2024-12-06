//! Routes for serving Scoria APK file downloads.
//!
//! A middleware prevents downloads from blocked countries. This includes all
//! anti-terrorism (AT) countries which are redundantly blocked by CloudFlare's
//! proxy. Non AT countries like France that have their own domestic regulations
//! that prevent distribution are blocked only at this level so a more
//! informative message can be displayed.

use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

use actix_web::http::header::ContentType;
use actix_web::{
    body::EitherBody,
    dev::{
        self, HttpServiceFactory, Service, ServiceRequest, ServiceResponse,
        Transform,
    },
    get, web, Error, HttpResponse, Responder,
};

static UNAVAILABLE_FILE: &str = include_str!(env!("UNAVAILABLE_FILE"));
static SCORIA_1_4_3: &[u8] = include_bytes!(env!("SCORIA_1_4_3"));
static SCORIA_1_4_2: &[u8] = include_bytes!(env!("SCORIA_1_4_2"));
static SCORIA_1_4_0: &[u8] = include_bytes!(env!("SCORIA_1_4_0"));
static SCORIA_1_3_0: &[u8] = include_bytes!(env!("SCORIA_1_3_0"));

const BLOCKED_COUNTRIES: [&str; 5] = ["CU", "IR", "KP", "SY", "FR"];
const CF_IPCOUNTRY_HEADER: &str = "CF-IPCountry";
const APK_CONTENT_TYPE_HEADER: (&str, &str) =
    ("content-type", "application/vnd.android.package-archive");

pub fn get_apk_file_services() -> impl HttpServiceFactory {
    web::scope("/download/apk")
        .wrap(CheckCountry)
        // NOTE: update this on updates:
        .service(web::redirect("/latest", "./Scoria_1.4.3.apk"))
        .service(latest_version)
        // NOTE: on updates, add new route for the new verison here:
        .service(scoria_1_4_3)
        .service(scoria_1_4_2)
        .service(scoria_1_4_0)
        .service(scoria_1_3_0)
}

/// Returns the latest version available for download as
/// "version_code,version_name"
#[get("/latest_version")]
async fn latest_version() -> impl Responder {
    HttpResponse::Ok().body("5,1.4.3") // NOTE: update this on updates
}

#[get("/Scoria_1.4.3.apk")]
async fn scoria_1_4_3() -> impl Responder {
    tracing::info!("Download of 1.4.3");
    HttpResponse::Ok()
        .insert_header(APK_CONTENT_TYPE_HEADER)
        .body(SCORIA_1_4_3)
}

#[get("/Scoria_1.4.2.apk")]
async fn scoria_1_4_2() -> impl Responder {
    tracing::info!("Download of 1.4.2");
    HttpResponse::Ok()
        .insert_header(APK_CONTENT_TYPE_HEADER)
        .body(SCORIA_1_4_2)
}

#[get("/Scoria_1.4.0.apk")]
async fn scoria_1_4_0() -> impl Responder {
    tracing::info!("Download of 1.4.0");
    HttpResponse::Ok()
        .insert_header(APK_CONTENT_TYPE_HEADER)
        .body(SCORIA_1_4_0)
}

#[get("/Scoria_1.3.0.apk")]
async fn scoria_1_3_0() -> impl Responder {
    tracing::info!("Download of 1.3.0");
    HttpResponse::Ok()
        .insert_header(APK_CONTENT_TYPE_HEADER)
        .body(SCORIA_1_3_0)
}

/// Checks if the country code of the connecting client is a country that is
/// blocked. If so, block the download by returning 403 forbidden with a brief
/// explanation of the unavailability. If CloudFlare failed to pass us the
/// country code, we allow the download.
///
/// The middleware constructs the 403 and returns early if the download is
/// blocked, otherwise is passes the request through to the routes.
///
/// Based on
/// https://github.com/actix/examples/blob/master/middleware/various/src/redirect.rs
pub struct CheckCountry;

impl<S, B> Transform<S, ServiceRequest> for CheckCountry
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckCountryMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckCountryMiddleware { service }))
    }
}
pub struct CheckCountryMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CheckCountryMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        if let Some(country_header) = request.headers().get(CF_IPCOUNTRY_HEADER)
        {
            let country_code = country_header.to_str().unwrap();
            if BLOCKED_COUNTRIES.contains(&country_code) {
                tracing::info!("Blocking download from {}", country_code);
                let response = HttpResponse::Forbidden()
                    .content_type(ContentType::html())
                    .body(UNAVAILABLE_FILE)
                    // constructed responses map to "right" body
                    .map_into_right_body();

                let (request, _pl) = request.into_parts();
                return Box::pin(async {
                    Ok(ServiceResponse::new(request, response))
                });
            }
        }

        let res = self.service.call(request);

        Box::pin(async move {
            // forwarded responses map to "left" body
            res.await.map(ServiceResponse::map_into_left_body)
        })
    }
}
