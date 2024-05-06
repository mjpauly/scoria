//! The Android emulator hangs when trying to load a track to replay, so we
//! instead run this program to update the location using adb's command line
//! interface.

use std::fs::File;
use std::io::BufReader;
use std::process::Command;

use gpx::{read, Gpx};

#[tokio::main]
async fn main() {
    // println!("Run dir: {}", std::env::current_dir().unwrap().display());

    // a track captured from the iOS simulator (freeway drive)
    let file =
        File::open("src/app/android/test/emu_geo/test_track.gpx").unwrap();
    let reader = BufReader::new(file);
    let gpx: Gpx = read(reader).unwrap();
    let points = &gpx.tracks[0].segments[0].points;

    let android_home = std::env::var("ANDROID_HOME").unwrap();
    let adb = format!("{android_home}/platform-tools/adb");

    let mut i = 0;
    loop {
        let x = format!("{}", points[i].point().x());
        let y = format!("{}", points[i].point().y());
        let output = Command::new(&adb)
            .arg("emu")
            .arg("geo")
            .arg("fix")
            .arg(&x)
            .arg(&y)
            .output()
            .expect("failed to execute process");
        println!("{x}, {y}: {}", String::from_utf8_lossy(&output.stdout));
        i = (i + 1) % points.len();
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await
    }
}
