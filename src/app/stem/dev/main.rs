//! Runs the app backend and frontend locally

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
    // copy our bundle into the expected location
    let bundle_path = "src/app/stem/front/dist.zip";
    println!("Bundle exists: {}", std::fs::metadata(bundle_path).is_ok());

    //DEBUG
    let paths = std::fs::read_dir("dev_instance/Bundle").unwrap();
    for path in paths {
        println!("Name: {}", path.unwrap().path().display())
    }

    let dest = "dev_instance/Bundle/dist.zip";
    println!(
        "Copy over success: {}",
        std::fs::copy(bundle_path, dest).is_ok()
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
}
