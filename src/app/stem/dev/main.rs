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
    let starting_n = 60;
    let mut data = stem::database::OSLocationData {
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp()
            - starting_n,
        latitude: 37.42984,
        longitude: -122.16945,
        horizontal_accuracy: random::<f64>() * 3.0 + 2.0,

        msl_altitude: 0.0,
        ellipsoid_altitude: 0.0,
        vertical_accuracy: 0.0,

        story_available: true,
        story: 0,

        speed: 0.0, // speed
        speed_accuracy: -1.0,
        course: -1.0, // course
        course_accuracy: -1.0,

        source_info_available: false,
        is_simulated_by_software: false,
        is_produced_by_accessory: false,
    };
    let mut vx = 0.;
    let mut vy = 0.;
    let mut i = 0;
    loop {
        if i > starting_n {
            sleep(Duration::from_millis(1000)).await;
        }
        i += 1;
        data.timestamp += 1;
        vx += (random::<f64>() - 0.5) / 10000.0;
        vy += (random::<f64>() - 0.5) / 10000.0;
        data.longitude =
            ((data.longitude + vx) + 180.0).rem_euclid(360.0) - 180.0;
        data.latitude = ((data.latitude + vy) + 90.0).rem_euclid(180.0) - 90.0;
        data.horizontal_accuracy = random::<f64>() * 3.0 + 2.0;
        data.msl_altitude += 1.0;
        data.vertical_accuracy -= 0.01;
        data.story += 1;
        data.speed = (vx * vx + vy * vy).sqrt();
        data.course = 180. - vy.atan2(vx).to_degrees();
        stem::core::log_location(data.clone()).await;
    }
}
