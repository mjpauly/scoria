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
pub fn gen_viz(records: Vec<common::Location>, marker: Marker) -> plotly::Plot {
    // filter out records where accuracy is worse (larger) than 20m
    let records_iter = records.iter().filter(|x| x.accuracy < 20.0);
    let lats: Vec<_> = records_iter.clone().map(|x| x.lat).collect();
    let lons: Vec<_> = records_iter.map(|x| x.lon).collect();

    // calculate where to put the center
    let mean_lat = lats.iter().sum::<f64>() / lats.len() as f64;
    let mean_lon = lons.iter().sum::<f64>() / lons.len() as f64;
    // TODO: calculate default zoom level

    let trace = ScatterMapbox::new(lats, lons).marker(marker);
    let layout = Layout::new()
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::StamenTerrain)
                .center(Center::new(mean_lat, mean_lon))
                .zoom(10),
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
pub fn empty_plot(marker: Marker) -> plotly::Plot {
    let trace =
        ScatterMapbox::new(Vec::<f64>::new(), Vec::<f64>::new()).marker(marker);
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
