//! Generate visualizations using plotly, embeded within yew components

use plotly::{
    common::{color, Marker},
    configuration::{Configuration, DisplayModeBar},
    layout::{Center, Mapbox, MapboxStyle, Margin},
    Layout, Plot, ScatterMapbox,
};

use crate::common;

/// Generate a map of data points and return the plot
pub fn gen_viz(
    records: Vec<common::Location>,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) -> plotly::Plot {
    // filter out records where accuracy is worse (larger) than 20m
    let records_iter = records.iter().filter(|x| x.accuracy < 20.0);
    let lats: Vec<_> = records_iter.clone().map(|x| x.lat).collect();
    let lons: Vec<_> = records_iter.map(|x| x.lon).collect();

    // calculate where to put the center
    let mean_lat = lats.iter().sum::<f64>() / lats.len() as f64;
    let mean_lon = lons.iter().sum::<f64>() / lons.len() as f64;

    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;

    let trace = ScatterMapbox::new(lats, lons).marker(
        Marker::new()
            .opacity(0.8)
            .color(color::Rgba::new(r, g, b, a)),
    );
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
