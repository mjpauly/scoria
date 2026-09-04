use std::io::BufWriter;
use std::{fs::File, path::PathBuf};

use jiff::Zoned;
use serde::Serialize;

use common::Location;

use crate::tz::datetime_fn_infallible;
use crate::ws_session::send_error_popup;

use super::ExportData;

pub(super) fn export(mut fname: PathBuf, data: ExportData) {
    fname.set_extension("csv");
    if let Err(e) = write_csv(&fname, data) {
        tracing::error!("Failed to write CSV file: {e}");
        send_error_popup("Failed to write CSV file.");
    }
}

fn write_csv(out_path: &PathBuf, data: ExportData) -> anyhow::Result<()> {
    let datetime_fn = datetime_fn_infallible();
    let records = data
        .records
        .iter()
        .map(|r| CSVLocationRecord::from_location(r, &datetime_fn))
        .collect::<Vec<CSVLocationRecord>>();
    let csv_file = File::create(out_path)?;
    let buf = BufWriter::new(csv_file);
    let mut wtr = csv::Writer::from_writer(buf);
    for rec in records {
        wtr.serialize(rec)?;
    }
    wtr.flush()?;
    Ok(())
}

/// Alias type for the fields we actually want to serialize into a CSV
/// Some fields are more like implementation details to the app that don't need
/// to be exported.
/// Other fields need to be transformed into the desired format, like the
/// timestamp, which is given both as a string and as unix epoch seconds.
#[derive(Serialize)]
struct CSVLocationRecord<'a> {
    timestamp: String,
    timestamp_as_seconds: i64,

    latitude: &'a f64,
    longitude: &'a f64,
    horizontal_accuracy: &'a f64,

    msl_altitude: &'a Option<f64>,
    ellipsoid_altitude: &'a Option<f64>,
    vertical_accuracy: &'a Option<f64>,
    story: &'a Option<i64>,

    speed: &'a Option<f64>,
    speed_accuracy: &'a Option<f64>,

    course: &'a Option<f64>,
    course_accuracy: &'a Option<f64>,
}

impl<'a> CSVLocationRecord<'a> {
    fn from_location(
        other: &'a Location,
        datetime_fn: &impl Fn(time::OffsetDateTime, common::LngLat) -> Zoned,
    ) -> Self {
        Self {
            timestamp: datetime_fn(other.timestamp, other.lnglat()).to_string(),
            timestamp_as_seconds: other.timestamp.unix_timestamp(),

            latitude: &other.latitude,
            longitude: &other.longitude,
            horizontal_accuracy: &other.horizontal_accuracy,

            msl_altitude: &other.msl_altitude,
            ellipsoid_altitude: &other.ellipsoid_altitude,
            vertical_accuracy: &other.vertical_accuracy,
            story: &other.story,

            speed: &other.speed,
            speed_accuracy: &other.speed_accuracy,

            course: &other.course,
            course_accuracy: &other.course_accuracy,
        }
    }
}
