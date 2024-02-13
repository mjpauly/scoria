//! Functions for exporting to common geo data formats

// use std::path::PathBuf;

use itertools::Itertools;
use time::format_description::well_known::Iso8601;

use crate::{app_state::AppState, database, geojson, paths, ws_session};
use common::{
    export_options::{ExportFormat, ExportOptions},
    filters::Filter,
    state::MapState,
    Location, TimeRange, ToFront,
};

pub mod csv;
pub mod gpx;

/// Exported selected map data (whatever is visible in the map) with the given
/// export options. Creates the desired file at Documents/track_export.{ext},
/// where the extension is determined by the desired filetype. The swift code
/// that creates the sharesheet finds the file by its name without the
/// extension, renames that part of the file to something like
/// Scoria_Track_2023-11-11_15-03-45.gpx
pub async fn export_selected() {
    let (map_state, export_opts) = {
        let app_state = AppState::global();
        let persistent_guard = app_state.persistent.lock().unwrap();
        let map_state = persistent_guard.front.as_ref().unwrap().map.clone();
        let export_opts = persistent_guard.front.as_ref().unwrap().export_opts;
        (map_state, export_opts)
    };

    let data = get_records_to_write(&map_state, &export_opts).await;
    // output path without the ".gpx", ".csv" extension
    let fname = paths::get_export_fname();
    match export_opts.format {
        ExportFormat::GPX => gpx::export(fname, data),
        ExportFormat::CSV => csv::export(fname, data, &map_state),
    }
    // notify the app wrapper to share the file
    AppState::global()
        .swift_messages
        .lock()
        .unwrap()
        .should_export_track = true;
    ws_session::send_message_to_front(ToFront::SwiftPoke);
}

/// Fetch records just the same as are visible in the map
async fn get_records_to_write(
    map_state: &MapState,
    export_opts: &ExportOptions,
) -> ExportData {
    let records = database::FilteredQuery::new()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(geojson::BOUND_EXPANSION))
        .limit(export_opts.max_points)
        .fetch_decimated()
        .await;
    ExportData {
        records,
        time_range: map_state.time_range,
        filters: map_state.filters.clone(),
        max_points: export_opts.max_points,
    }
}

/// The points to export along with associated metadata to embed in the exported
/// file, if possible.
pub struct ExportData {
    records: Vec<Location>,
    time_range: TimeRange,
    filters: Vec<Filter>,
    max_points: u64,
}

impl ExportData {
    fn comment(&self) -> String {
        let time_comment = format!(
            "Time range: {} to {}",
            Self::format_time(&self.time_range.start),
            Self::format_time(&self.time_range.start)
        );
        let filter_comment = format!(
            "Excludes points where: [{}]",
            self.filters
                .iter()
                .filter(|f| f.enabled)
                .map(|f| format!("{} {} {}", f.datastream, f.op, f.threshold))
                .join(", ")
        );
        let count_comment = format!(
            "Num points: {} (capped to {})",
            self.records.len(),
            self.max_points
        );
        format!("{time_comment},\n{filter_comment},\n{count_comment}")
    }

    /// Format the time. Since this is in the comment we don't convert to UTC.
    fn format_time(t: &time::OffsetDateTime) -> String {
        t.format(&Iso8601::DEFAULT).unwrap()
    }
}
