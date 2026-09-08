use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::configuration::DatabaseSettings;
use crate::configuration::Settings;
use crate::configuration::TurnstileSettings;
use crate::routes::*;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(
        configuration: Settings,
    ) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            configuration.application.base_url,
            configuration.turnstile,
        )?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // This function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

// We need to define a wrapper type in order to retrieve the URL
// in the `subscribe` handler.
// Retrieval from the context, in actix-web, is type-based: using
// a raw `String` would expose us to conflicts.
pub struct ApplicationBaseUrl(pub String);

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    base_url: String,
    turnstile: TurnstileSettings,
) -> Result<Server, std::io::Error> {
    // capture db_pool with `move`, but make sure it is cloned for each
    // actix worker (it is just a referenced-counted pointer)
    let db_pool = web::Data::new(db_pool);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let turnstile = web::Data::new(turnstile);
    // Outbound client for Turnstile verification. Bounded so a slow
    // Cloudflare can't hold a worker for long.
    let http_client = web::Data::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    );
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .service(get_static_file_services())
            .service(contact_form_submitted)
            .service(get_android_services())
            .service(get_apk_file_services())
            .app_data(db_pool.clone())
            .app_data(base_url.clone())
            .app_data(turnstile.clone())
            .app_data(http_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
