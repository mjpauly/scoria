//! Generate visualizations using plotly, embeded within yew components

pub use plotly::common::{color, Marker};
use plotly::{
    configuration::{Configuration, DisplayModeBar},
    layout::{Center, Mapbox, MapboxStyle, Margin},
    Layout, Plot, ScatterMapbox,
};

use crate::common;

/// Generate a map of data points and return the plot
#[allow(dead_code)]
pub fn map_plot(
    records: Vec<common::Location>,
    marker: Marker,
) -> plotly::Plot {
    // filter out records where accuracy is worse (larger) than 20m
    let records_iter = records.iter().filter(|x| x.accuracy < 20.0);
    let lats: Vec<_> = records_iter.clone().map(|x| x.lat).collect();
    let lons: Vec<_> = records_iter.map(|x| x.lon).collect();

    let (lat_center, lon_center, zoom);
    let trace;
    if records.len() > 0 {
        // calculate where to put the center
        (lat_center, lon_center, zoom) = get_center_and_zoom(&lats, &lons);
        trace = ScatterMapbox::new(lats, lons).marker(marker);
    } else {
        (lat_center, lon_center, zoom) = (0., 0., 0);
        // If we don't have a data point, mapbox-gl-js complains about "there is
        // already a source with this ID"
        trace = ScatterMapbox::new(vec![0.], vec![0.]).marker(marker);
    }

    let layout = Layout::new()
        // transparent paper: no white flashes on plot load
        .paper_background_color(color::Rgba::new(0, 0, 0, 0.))
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::StamenTerrain)
                .center(Center::new(lat_center, lon_center))
                .zoom(zoom),
        );
    let config = Configuration::new()
        .responsive(true)
        .display_logo(false)
        .display_mode_bar(DisplayModeBar::False);

    let mut plot = Plot::new();
    plot.use_local_plotly();
    plot.add_trace(trace);
    plot.set_layout(layout);
    plot.set_configuration(config);
    plot
}

/// Generate an empty plot layout without traces
#[allow(dead_code)]
pub fn empty_plot(marker: Marker) -> plotly::Plot {
    // If we don't have a data point, mapbox-gl-js complains about "there is
    // already a source with this ID"
    let trace = ScatterMapbox::new(vec![0.0], vec![0.0]).marker(marker);
    let layout = Layout::new()
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(Mapbox::new().style(MapboxStyle::StamenTerrain).zoom(1));
    let config = Configuration::new()
        .responsive(true)
        .display_logo(false)
        .display_mode_bar(DisplayModeBar::False);

    let mut plot = Plot::new();
    plot.use_local_plotly();
    plot.add_trace(trace);
    plot.set_layout(layout);
    plot.set_configuration(config);
    plot
}

/// Floats don't implement Ord, so we have to do this
fn float_min(vals: &Vec<f64>) -> f64 {
    *vals
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}
fn float_max(vals: &Vec<f64>) -> f64 {
    *vals
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}

/// Recenter a map. Make sure there is at least one data point in the vectors
/// when calling this.
fn get_center_and_zoom(lats: &Vec<f64>, lons: &Vec<f64>) -> (f64, f64, u8) {
    let lat_center = (float_max(&lats) + float_min(&lats)) / 2.;
    let lon_center = (float_max(&lons) + float_min(&lons)) / 2.;
    let lon_range = float_max(&lons) - float_min(&lons);
    let mut zoom = (360. / lon_range).log2().floor() - 1.;
    if zoom > 16. {
        zoom = 16.;
    }
    if zoom < 0. {
        zoom = 0.;
    }
    let zoom = zoom as u8;
    (lat_center, lon_center, zoom)
}
