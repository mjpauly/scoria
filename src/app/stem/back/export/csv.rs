use std::io::BufWriter;
use std::{fs::File, path::PathBuf};

use common::state::MapState;
use serde::Serialize;
use time::format_description::well_known::Iso8601;

use common::Location;

use super::ExportData;

pub fn export(mut fname: PathBuf, data: ExportData, map_state: &MapState) {
    fname.set_extension("csv");
    if let Err(e) = write_csv(&fname, data, map_state) {
        tracing::error!("Failed to write CSV file: {e}");
    }
}

fn write_csv(
    out_path: &PathBuf,
    data: ExportData,
    map_state: &MapState,
) -> anyhow::Result<()> {
    let local_offset = map_state.time_range.start.offset();
    let records = data
        .records
        .iter()
        .map(|r| CSVLocationRecord::from_location(r, &local_offset))
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
    pub timestamp: String,
    pub timestamp_as_seconds: i64,

    pub latitude: &'a f64,
    pub longitude: &'a f64,
    pub horizontal_accuracy: &'a f64,

    pub msl_altitude: &'a Option<f64>,
    pub ellipsoid_altitude: &'a Option<f64>,
    pub vertical_accuracy: &'a Option<f64>,
    pub story: &'a Option<i64>,

    pub speed: &'a Option<f64>,
    pub speed_accuracy: &'a Option<f64>,

    pub course: &'a Option<f64>,
    pub course_accuracy: &'a Option<f64>,
}

impl<'a> CSVLocationRecord<'a> {
    fn from_location(
        other: &'a Location,
        local_offset: &time::UtcOffset,
    ) -> Self {
        Self {
            timestamp: other
                .timestamp
                .to_offset(*local_offset) // TODO: check
                .format(&Iso8601::DEFAULT)
                .unwrap(),
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
