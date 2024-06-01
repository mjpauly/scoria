//! Colormap calculations for data.

use common::{
    cmaps::{Cmap, CmapParams},
    float,
    map_style::ColoredDataStream,
    Location,
};
use itertools::Itertools;

use super::distance::distance_between_locations;

/// Calculate the cmap parameters and the data values that determine the color
/// given a slice of locations.
pub fn get_cmap_params(
    colored_datastream: &ColoredDataStream,
    // bool indicates if point contributes to the cmap, or will be hidden
    records: &[(&Location, bool)],
    offset: &time::UtcOffset,
) -> Option<(CmapParams, Vec<Option<f64>>)> {
    if !colored_datastream.is_some() {
        return None;
    }
    let datavals = get_colored_data_vals(colored_datastream, records, offset);
    let only_valid_data =
        datavals.iter().filter_map(|x| *x).collect::<Vec<_>>();
    let mut params = CmapParams::default();
    if *colored_datastream == ColoredDataStream::Course {
        params.cmax = 360.;
        params.cmap = Cmap::Twilight;
    } else if *colored_datastream == ColoredDataStream::TimeOfDay {
        params.cmax = 24. * 60. * 60.;
        params.cmap = Cmap::TwilightShifted;
    } else {
        params.cmin = float::min(&only_valid_data);
        params.cmax = float::max(&only_valid_data);
    }
    debug_assert_eq!(records.len(), datavals.len());
    Some((params, datavals))
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
        delta_color_vals(records, |(a, b)| {
            Some(
                distance_between_locations(a, b)
                    / (b.timestamp - a.timestamp).as_seconds_f64(),
            )
        })
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
fn delta_color_vals(
    records: &[(&Location, bool)],
    data_fn: impl Fn((&Location, &Location)) -> Option<f64>,
) -> Vec<Option<f64>> {
    let mut out: Vec<_> = records
        .iter()
        .tuple_windows::<(_, _)>()
        // only if first data point, or its line, will be visible
        .map(|(a, b)| a.1.then(|| data_fn((a.0, b.0))).flatten())
        .collect();
    if out.len() < records.len() {
        out.push(None); // final point is uncolored
    }
    out
}
