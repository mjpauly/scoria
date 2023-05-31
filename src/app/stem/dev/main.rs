//! Runs the app backend and frontend locally
//!
//! To print debug log messages set RUST_LOG to the desired log level like so:
//! RUST_LOG=info ibazel run :dev
//!

use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};

use rand::random;

extern crate stem;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init(); // sets output verbosity based on RUST_LOG env var

    println!("Run dir: {}", std::env::current_dir().unwrap().display());

    stem::local::local_setup_with_dev_db("dev_fs/", 8081).await;

    // Spawn our data generator
    tokio::spawn(data_generator());

    println!("Running server until ctrl-c is sent.");
    // If we're using ibazel it will upgrade our SIGINT (ctrl-c) to SIGTERM, so
    // we need to listen to both signals.
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => log::info!("\nReceived SIGINT"),
        _ = sigterm.recv() => log::info!("\nReceived SIGTERM"),
    }

    println!("Shutting down the server.");
    // `true` tells actix to do a graceful shutdown
    // server_handle.stop(true).await;
    // calling this now seems to not kill the server when expected? this now
    // has the desired behavior when not calling this funtion.

    Ok(())
}

async fn data_generator() {
    let mut lat = 37.42984;
    // let mut lat = 85.;
    let mut lon = -122.16945;
    // let ca = lat + 0.001;  // center of circular-ish track
    // let co = lon + 0.001;
    let mut vx = 0.;
    let mut vy = 0.;
    let mut i = 0;
    let starting_n = 60;
    let mut t = time::OffsetDateTime::now_utc().unix_timestamp() - starting_n;
    loop {
        if i > starting_n {
            sleep(Duration::from_millis(1000)).await;
        }
        i += 1;
        t += 1;
        vx += (random::<f64>() - 0.5) / 10000.0;
        vy += (random::<f64>() - 0.5) / 10000.0;
        lon = ((lon + vx) + 180.0).rem_euclid(360.0) - 180.0;
        lat = ((lat + vy) + 90.0).rem_euclid(180.0) - 90.0;
        stem::core::log_location(
            lat,
            lon,
            random::<f64>() * 3.0 + 2.0, // accuracy
            (vx * vx + vy * vy).sqrt(),  // speed
            180. - vy.atan2(vx).to_degrees(), // course
            t,
        )
        .await;
    }
}
