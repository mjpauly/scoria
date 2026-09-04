//! Runs the app backend and frontend locally
//!
//! To print debug log messages set RUST_LOG to the desired log level like so:
//! RUST_LOG=info ibazel run :dev
//!
//! Synthetic-data probe mode, used by the memory probe harness
//! (doc/decimation/memory-limits.md, "Probe harness"):
//! bazel run :dev -- --synth 100000 --shape walk --style lines --port 0

mod synth;

use std::path::PathBuf;

use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};

use rand::random;

extern crate stem;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    println!("Run dir: {}", std::env::current_dir().unwrap().display());

    // --port overrides the mode's default, e.g. to run a second instance
    let args: Vec<String> = std::env::args().collect();
    let port: Option<u16> = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .map(|p| p.parse().expect("port"));

    match synth::SynthConfig::from_args() {
        Some(cfg) => {
            // Must be in the env before the first map query reads it
            std::env::set_var(
                "STEM_DECIMATION_THRESHOLD",
                cfg.threshold.to_string(),
            );
            // Fresh empty database, not the (large) dev db; port 0 lets
            // the OS pick, reported in the PROBE_READY line
            let port =
                stem::local::local_setup("dev_fs/", port.unwrap_or(0)).await;
            synth::seed_front_state(&cfg);
            synth::insert_synth_data(&cfg).await;
            // Marker for the probe harness: server up, data loaded
            println!("PROBE_READY port={port}");
        }
        None => {
            let port = port.unwrap_or(8081);
            stem::local::local_setup_with_dev_db("dev_fs/", port).await;

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
        }
    }

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
        latitude: 37.8,
        longitude: -122.5,
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
