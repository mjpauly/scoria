//! Colormap calculations for data.

use common::{
    cmaps::{Cmap, CmapParams},
    float,
    map_style::ColoredDataStream,
    Location,
};
use itertools::Itertools;
use tracing::{instrument, Level};

use crate::metrics::dashboard::update_timeseries_plot_data;

use super::{
    distance::distance_between_locations,
    dwells::{dwell_score, short_dwell_detect},
    speed::avg_speed,
};

/// Calculate the cmap parameters and the data values that determine the color
/// given a slice of locations.
pub fn get_cmap_data(
    colored_datastream: &ColoredDataStream,
    // bool indicates if point contributes to the cmap, or will be hidden
    records: &[(&Location, bool)],
    offset: &time::UtcOffset,
) -> Option<(CmapParams, Vec<Option<f64>>)> {
    if !colored_datastream.is_some() {
        return None;
    }
    let cmap_vals = get_colored_data_vals(colored_datastream, records, offset);
    let mut params = CmapParams::default();
    if *colored_datastream == ColoredDataStream::Course {
        params.cmax = 360.;
        params.cmap = Cmap::Twilight;
    } else if *colored_datastream == ColoredDataStream::TimeOfDay {
        params.cmax = 24. * 60. * 60.;
        params.cmap = Cmap::TwilightShifted;
    } else if *colored_datastream == ColoredDataStream::ShortDwellDetection
        || *colored_datastream == ColoredDataStream::LongDwellDetection
        || *colored_datastream == ColoredDataStream::DwellScore
    {
        params.cmax = 1.0
    } else {
        let only_valid_data = cmap_vals.iter().filter_map(|x| (*x).as_ref());
        params.cmin = float::min(only_valid_data.clone());
        params.cmax = float::max(only_valid_data);
    }
    debug_assert_eq!(records.len(), cmap_vals.len());
    update_timeseries_plot_data(
        records.iter().map(|r| r.0),
        &cmap_vals,
        colored_datastream,
    );
    Some((params, cmap_vals))
}

/// Computes the colored data values for a slice of locations, returning a
/// vector of the same length with floats for
pub fn get_colored_data_vals(
    colored_datastream: &ColoredDataStream,
    records: &[(&Location, bool)],
    offset: &time::UtcOffset,
) -> Vec<Option<f64>> {
    if *colored_datastream == ColoredDataStream::DistanceDelta {
        delta_color_vals(records, |(a, b)| {
            Some(distance_between_locations(a, b))
        })
    } else if *colored_datastream == ColoredDataStream::TimeDelta {
        delta_color_vals(records, |(a, b)| {
            Some((b.timestamp - a.timestamp).as_seconds_f64())
        })
    } else if *colored_datastream == ColoredDataStream::AvgSpeed {
        delta_color_vals(records, |(a, b)| Some(avg_speed(a, b)))
    } else if *colored_datastream == ColoredDataStream::ShortDwellDetection {
        delta_color_vals(records, |(a, b)| {
            Some(dwell_to_colorval(short_dwell_detect(a, b)))
        })
    } else if *colored_datastream == ColoredDataStream::LongDwellDetection {
        long_dwell_colors(records)
    } else if *colored_datastream == ColoredDataStream::DwellScore {
        dwell_score(records)
    } else {
        debug_assert!(colored_datastream.is_point_coloring());
        records
            .iter()
            .map(|(l, visible)| {
                visible
                    .then(|| colored_datastream.get_stream(l, offset))
                    .flatten()
            })
            .collect()
    }
}

/// Compute pair-wise data color values given a function to evaluate on each
/// pair of locations.
#[instrument(skip_all, level = Level::TRACE)]
fn delta_color_vals(
    records: &[(&Location, bool)],
    data_fn: impl Fn((&Location, &Location)) -> Option<f64>,
) -> Vec<Option<f64>> {
    let mut out: Vec<_> = records
        .iter()
        .tuple_windows::<(_, _)>()
        // only if the line, which is associated with the first data point, will
        // be visible
        .map(|(a, b)| a.1.then(|| data_fn((a.0, b.0))).flatten())
        .collect();
    if out.len() < records.len() {
        out.push(None); // final point is uncolored
    }
    out
}

/// Make dwells a light color, and movement dark.
fn dwell_to_colorval(val: bool) -> f64 {
    if val {
        1.0
    } else {
        0.0
    }
}

const LONG_DWELL_THRESHOLD: f64 = 10.0;

fn long_dwell_colors(records: &[(&Location, bool)]) -> Vec<Option<f64>> {
    let dwell_scores = dwell_score(records);
    let chunker = dwell_scores
        .iter()
        .map(|d| d.map(|v| v < LONG_DWELL_THRESHOLD))
        .chunk_by(|t| *t);
    let mut out = vec![];
    let mut c = 0;
    for (k, chunk) in chunker.into_iter() {
        match k {
            Some(true) => {
                let cval = c as f64 / 10.0 + 0.5;
                out.extend(chunk.map(|_| Some(cval)));
                c = (c + 1) % 6;
            }
            Some(false) => out.extend(chunk.map(|_| Some(0.0))),
            None => out.extend(chunk.map(|_| None)),
        }
    }
    out
}
