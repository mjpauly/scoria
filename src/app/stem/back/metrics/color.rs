//! Colormap calculations for data.

use common::{
    cmaps::{Cmap, CmapParams, Cminmax},
    float,
    map_style::ColoredDataStream,
};
use itertools::Itertools;
use tracing::{instrument, Level};

use crate::{database::NarrowPoint, tz::location_datetime_fn};

use super::dwells::{
    dwell_score_segment, long_dwell_threshold, segment_on_visibility,
    short_dwell_detect,
};

/// The database column the map query fetches into NarrowPoint::field1 for
/// this coloring: the point colorings read their value from it, and the
/// dwell colorings need ellipsoid_altitude for their distances
/// (distance_between_narrow_points). Time and TimeOfDay come from the
/// timestamp (and position) alone.
pub fn field1_column(ds: &ColoredDataStream) -> Option<&'static str> {
    match ds {
        ColoredDataStream::None
        | ColoredDataStream::Time
        | ColoredDataStream::TimeOfDay => None,
        ColoredDataStream::HorizAccuracy => Some("horizontal_accuracy"),
        ColoredDataStream::Altitude => Some("msl_altitude"),
        ColoredDataStream::VertAccuracy => Some("vertical_accuracy"),
        ColoredDataStream::Speed => Some("speed"),
        ColoredDataStream::SpeedAccuracy => Some("speed_accuracy"),
        ColoredDataStream::Course => Some("course"),
        ColoredDataStream::CourseAccuracy => Some("course_accuracy"),
        ColoredDataStream::ShortDwellDetection
        | ColoredDataStream::LongDwellDetection
        | ColoredDataStream::DwellScore => Some("ellipsoid_altitude"),
    }
}

/// Calculate the cmap parameters and the data values that determine the color
/// given a slice of locations.
pub fn get_cmap_data(
    colored_datastream: &ColoredDataStream,
    // bool indicates if point contributes to the cmap, or will be hidden.
    // The records' field1 must hold field1_column(colored_datastream).
    records: &[(&NarrowPoint, bool)],
) -> Option<(CmapParams, Vec<Option<f64>>)> {
    if !colored_datastream.is_some() {
        return None;
    }
    let cmap_vals = get_colored_data_vals(colored_datastream, records);
    let mut params = CmapParams::default();
    if *colored_datastream == ColoredDataStream::Course {
        params.cminmax = Some(Cminmax {
            cmin: 0.,
            cmax: 360.,
        });
        params.cmap = Cmap::Twilight;
    } else if *colored_datastream == ColoredDataStream::TimeOfDay {
        params.cminmax = Some(Cminmax {
            cmin: 0.,
            cmax: 24. * 60. * 60.,
        });
        params.cmap = Cmap::TwilightShifted;
    } else if *colored_datastream == ColoredDataStream::ShortDwellDetection
        || *colored_datastream == ColoredDataStream::LongDwellDetection
    {
        params.cminmax = Some(Cminmax { cmin: 0., cmax: 1. });
    } else {
        let only_valid_data = cmap_vals.iter().filter_map(|x| (*x).as_ref());
        params.cminmax = float::min(only_valid_data.clone())
            .zip(float::max(only_valid_data))
            .map(|(cmin, cmax)| Cminmax { cmin, cmax });
    }
    debug_assert_eq!(records.len(), cmap_vals.len());
    Some((params, cmap_vals))
}

/// Computes the colored data values for a slice of locations, returning a
/// vector of the same length with floats for
pub fn get_colored_data_vals(
    colored_datastream: &ColoredDataStream,
    records: &[(&NarrowPoint, bool)],
) -> Vec<Option<f64>> {
    if *colored_datastream == ColoredDataStream::ShortDwellDetection {
        delta_color_vals(records, |(a, b)| {
            Some(dwell_to_colorval(short_dwell_detect(a, b)))
        })
    } else if *colored_datastream == ColoredDataStream::LongDwellDetection {
        long_dwell_colors(records)
    } else if *colored_datastream == ColoredDataStream::DwellScore {
        segment_on_visibility(records)
            .iter()
            .flat_map(|s| dwell_score_segment(s))
            .collect()
    } else {
        debug_assert!(colored_datastream.is_point_coloring());
        let tod = location_tod_fn();
        let value = |l: &NarrowPoint| match colored_datastream {
            ColoredDataStream::Time => {
                Some(l.timestamp.unix_timestamp() as f64)
            }
            ColoredDataStream::TimeOfDay => tod(l),
            // point colorings read the column fetched for them (field1_column)
            _ => l.field1,
        };
        records
            .iter()
            .map(|(l, visible)| visible.then(|| value(l)).flatten())
            .collect()
    }
}

/// Return a fn that converts a point to time of day in seconds
pub fn location_tod_fn() -> impl Fn(&NarrowPoint) -> Option<f64> {
    let f = location_datetime_fn();
    move |x: &NarrowPoint| {
        f(x.timestamp, x.lnglat()).ok().map(|zdt| {
            zdt.hour() as f64 * 60. * 60.
                + zdt.minute() as f64 * 60.
                + zdt.second() as f64
        })
    }
}

/// Compute pair-wise data color values given a function to evaluate on each
/// pair of locations.
#[instrument(skip_all, level = Level::TRACE)]
fn delta_color_vals(
    records: &[(&NarrowPoint, bool)],
    data_fn: impl Fn((&NarrowPoint, &NarrowPoint)) -> Option<f64>,
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

#[instrument(skip_all, level = Level::TRACE)]
fn long_dwell_colors(records: &[(&NarrowPoint, bool)]) -> Vec<Option<f64>> {
    let segmented = segment_on_visibility(records);
    let is_dwells = segmented
        .iter()
        .flat_map(|s| long_dwell_threshold(dwell_score_segment(s).into_iter()));
    let chunker = is_dwells.chunk_by(|is_dwell| *is_dwell);
    let mut out = vec![];
    let mut c = 0;
    for (k, chunk) in chunker.into_iter() {
        match k {
            true => {
                let cval = c as f64 / 10.0 + 0.5;
                out.extend(chunk.map(|_| Some(cval)));
                c = (c + 1) % 6;
            }
            false => out.extend(chunk.map(|_| Some(0.0))),
        }
    }
    out
}
