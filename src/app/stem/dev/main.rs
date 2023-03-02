//! Runs the app backend and frontend locally

use std::fs;
use std::os::unix::fs::PermissionsExt;

use tokio::signal::unix::{signal, SignalKind};

extern crate stem;

#[tokio::main]
async fn main() {
    let dirs = stem::local::create_subdirs("dev_instance/");
    let paths_to_set = stem::paths::Paths::new(
        Some(dirs[0].clone()),
        Some(dirs[1].clone()),
        Some(dirs[2].clone()),
        Some(dirs[3].clone()),
    );

    // Copy our bundle into the expected location
    let bundle_path = "src/app/stem/front/dist.zip";
    let dest = "dev_instance/Bundle/dist.zip";
    println!("Bundle exists: {}", fs::metadata(bundle_path).is_ok());
    println!("Copying...");
    // This will fail if we didn't set writable permissions last time.
    fs::copy(bundle_path, dest).unwrap();

    // dist.zip is a genrule output, so it is read-only by default. We change it
    // to writable so that future invocations of fs::copy will work if the
    // sandbox is not cleared
    fs::set_permissions(dest, fs::Permissions::from_mode(0o666)).unwrap();
    println!(
        "Copied and set dist.zip permissions to: {:#o} (hopefully 0o100666)",
        fs::metadata(dest).unwrap().permissions().mode()
    );

    // Finish startup now that our bundle is in the right spot
    let server_handle = stem::init(paths_to_set)
        .await
        .expect("Could not start server");

    println!("Running server until ctrl-c is sent...");
    // If we're using ibazel it will upgrade our SIGINT (ctrl-c) to SIGTERM, so
    // we need to listen to both signals.
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv() => println!("\nReceived SIGINT"),
        _ = sigterm.recv() => println!("\nReceived SIGTERM"),
    }

    println!("Shutting down the server gracefully");
    // `true` tells actix to do a graceful shutdown
    server_handle.stop(true).await;
    println!("Server shut down, exiting.");
}
