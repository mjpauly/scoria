//! Runs the app backend and frontend locally
//!
//! To print debug log messages set RUST_LOG to the desired log level like so:
//! RUST_LOG=info ibazel run :dev
//!

use std::fs;
use std::os::unix::fs::PermissionsExt;

use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};

use rand::random;

extern crate stem;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init(); // sets output verbosity based on RUST_LOG env var

    let dev_fs = "dev_fs/"; // our iOS-like filesystem for running locally
    if fs::metadata(dev_fs).is_ok() {
        // clean it out if it's left over from last time
        fs::remove_dir_all(dev_fs)?;
    }
    fs::create_dir(dev_fs)?;

    let paths_to_set = stem::local::create_subdirs(dev_fs);

    // Copy our bundle into the expected location
    let bundle_path = "src/app/stem/front/dist.zip";
    let dest = std::path::Path::new(dev_fs).join("Bundle/dist.zip");
    fs::copy(bundle_path, dest.clone())?;
    // dist.zip is a bazel output, so it is read-only by default. We change it
    // to writable so that future invocations of fs::copy will work even if the
    // sandbox is not cleared, and also so that the read-only permissions don't
    // propagate further
    fs::set_permissions(dest.clone(), fs::Permissions::from_mode(0o666))?;
    log::info!(
        "Copied dist.zip and set permissions to: {:#o} (hopefully 0o100666)",
        fs::metadata(dest.clone())?.permissions().mode()
    );

    // Finish startup now that our bundle is in the right spot
    let _server_handle = stem::init(paths_to_set)
        .await
        .expect("Could not start server");

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
    let mut lon = -122.16945;
    loop {
        sleep(Duration::from_millis(1000)).await;
        lon = (lon + (random::<f64>() - 0.5) / 10000.0) % 180.0;
        lat = (lat + (random::<f64>() - 0.5) / 10000.0) % 180.0;
        stem::top::log_location(
            lat,
            lon,
            random::<f64>() * 5.0,   // accuracy
            random::<f64>() * 0.5,   // speed
            random::<f64>() * 360.0, // course
            time::OffsetDateTime::now_utc().unix_timestamp(),
        )
        .await;
    }
}
