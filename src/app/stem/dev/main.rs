//! Runs the app backend and frontend locally
//!
//! To print debug log messages set RUST_LOG to the desired log level like so:
//! RUST_LOG=info ibazel run :dev
//!

use std::path::PathBuf;

use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};

use rand::random;

extern crate stem;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    println!("Run dir: {}", std::env::current_dir().unwrap().display());

    stem::local::local_setup_with_dev_db("dev_fs/", 8081).await;

    // stem::core::handle_url_scheme("scoria://place?name=Ferry+Building&lng=-122.39339582391952&lat=37.79552680112931&icon=%E2%9B%B4%EF%B8%8F".into());
    // stem::core::handle_url_scheme("scoria://place?name=Ferry+Building&lng=-122.39339582391952&lat=37.79552680112931&icon=%E2%9B%B4%EF%B8%8F&lists%5B0%5D=To+go&tags%5B0%5D%5B0%5D=Website&tags%5B0%5D%5B1%5D=https%3A%2F%2Fwww.ferrybuildingmarketplace.com%2F".into());

    // tokio::spawn(async {
    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // stem::import::pins_geojson::import_pins(
    // "dev_fs/tmp/saved_places.json".into(),
    // )
    // .await;
    // });

    // Spawn our data generator
    tokio::spawn(data_generator());

    // import_mounted_db().await;

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
    // let update_rate: u64 = 1; // seconds between updates
    let update_rate: u64 = 2; // seconds between updates
    let starting_n = 200;
    // let starting_n = 30;
    // let starting_n = 1;
    let mut data = stem::database::OSLocationData {
        timestamp: jiff::Timestamp::now().as_second()
            - (starting_n * update_rate as i64),
        // latitude: 35.68697,
        // longitude: 139.70140,
        latitude: 37.5,
        longitude: -122.3,
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
    let mut vx = 0.0;
    let mut vy = 0.0;
    let mut i = 0;
    loop {
        if i > starting_n {
            sleep(Duration::from_secs(update_rate)).await;
        }
        i += 1;
        data.timestamp += update_rate as i64;
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
        data.course = (vx.atan2(vy).to_degrees() + 360.0).rem_euclid(360.0);
        stem::core::log_location(data.clone()).await;
    }
}

/// Note: must add dep on import_seg.db to stem/BUILD file.
#[allow(unused)]
async fn import_mounted_db() {
    let import_path = PathBuf::from("src/app/stem/db_full/import_seg.db");
    stem::database::mounted::mount_db(import_path).await;
}
