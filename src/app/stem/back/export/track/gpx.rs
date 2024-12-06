//! Exports selected data as a GPX

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use geo::Point;
use gpx::{Gpx, GpxVersion, Metadata, Track, TrackSegment, Waypoint};

use common::Location;

use crate::ws_session::send_error_popup;

use super::ExportData;

pub(super) fn export(mut fname: PathBuf, data: ExportData) {
    let gpx = make_gpx(data);
    fname.set_extension("gpx");
    if let Err(e) = write_gpx(&fname, gpx) {
        tracing::error!("Failed to write GPX file: {e}");
        send_error_popup("Failed to write GPX file.");
    }
}

fn make_gpx(data: ExportData) -> Gpx {
    let track_segment = TrackSegment {
        points: data.records.iter().map(location_to_waypoint).collect(),
    };
    let mut track = Track::new();
    track.segments = vec![track_segment];
    // attach details about the data filtering in the comment on the track
    let mut comment = data.comment();
    // note that our HDOP and VDOP fields are not canonical DOP values.
    comment.push_str(
        ",\nNote: HDOP and VDOP are not standard DOP values. \
        They are estimated horizontal and vertical accuracy in meters.",
    );
    track.comment = Some(comment);
    // file-level metadata fields for details about the file's generation
    let metadata = Metadata {
        links: vec![gpx::Link {
            href: "https://scoria.info".to_string(),
            text: None,
            type_: None,
        }],
        time: Some(time::OffsetDateTime::now_utc().into()),
        ..Default::default()
    };
    Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("Scoria".to_string()),
        metadata: Some(metadata),
        waypoints: vec![],
        tracks: vec![track],
        routes: vec![],
    }
}

fn location_to_waypoint(rec: &Location) -> Waypoint {
    let point = Point::new(rec.longitude, rec.latitude);
    let mut wpt = Waypoint::new(point);
    wpt.time = Some(rec.timestamp.into());
    // elevation above the WGS84 ellipsoid
    wpt.elevation = rec.ellipsoid_altitude;
    // geoidheight is just the MSL height aboce the geoid, which we can derive
    // by subtracting our MSL altitude from our ellipsoid altitude
    wpt.geoidheight = rec
        .msl_altitude
        .zip(rec.ellipsoid_altitude)
        .map(|(msl, ell)| msl - ell);
    wpt.speed = rec.speed;
    // for horizontal and vertical accuracy we violate the GPX spec a little,
    // since these are not the same units as normal HDOP and PDOP
    wpt.hdop = Some(rec.horizontal_accuracy);
    wpt.vdop = rec.vertical_accuracy;
    wpt
}

fn write_gpx(out_path: &PathBuf, gpx: Gpx) -> anyhow::Result<()> {
    let gpx_file = File::create(out_path)?;
    let buf = BufWriter::new(gpx_file);
    gpx::write(&gpx, buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use common::TimeRange;

    use crate::database::{log_location, tests::get_test_data, FilteredQuery};
    use crate::local::test_setup;

    use super::{export, ExportData, PathBuf};

    #[tokio::test]
    async fn test_write_gpx() {
        test_setup("test_write_gpx/").await;
        for i in 0..10 {
            log_location(get_test_data(i)).await.unwrap();
        }
        let time_range = TimeRange {
            start: jiff::Timestamp::from_second(0).unwrap(),
            end: jiff::Timestamp::from_second(1000).unwrap(),
        };
        let records = FilteredQuery::builder()
            .time_range(time_range)
            .limit(1_000)
            .build()
            .fetch_first_n()
            .await;

        let data = ExportData {
            records,
            time_range,
            filters: vec![],
            max_points: 10_000,
        };
        let fname = PathBuf::from("test_export");
        export(fname, data);

        println!("Run dir: {}", std::env::current_dir().unwrap().display());
    }
}
