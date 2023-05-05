//! Generate visualizations using plotly, embeded within yew components

use plotly::{
    color::Rgba,
    common::Marker,
    configuration::{Configuration, DisplayModeBar},
    layout::{Center, Mapbox, MapboxStyle, Margin, NamedMapboxStyle},
    Layout, Plot, ScatterMapbox,
};

use crate::plots::plotly_binds::MapboxRelayoutData;

/// Generate a map of data points and return the plot
pub fn map_plot(
    records: Vec<&common::Location>,
    marker: Marker,
    mapbox_style: MapboxStyle,
    relayout_data: Option<MapboxRelayoutData>,
) -> plotly::Plot {
    let lats: Vec<_> = records.iter().map(|x| x.lat).collect();
    let lons: Vec<_> = records.iter().map(|x| x.lon).collect();

    let (lat_center, lon_center, zoom, bearing, pitch);
    if let Some(data) = relayout_data {
        (lat_center, lon_center, zoom, bearing, pitch) = (
            data.center.lat,
            data.center.lon,
            data.zoom.round() as u8,
            data.bearing,
            data.pitch,
        );
    } else {
        // new plot: calculate where to put the center and zoom
        (lat_center, lon_center, zoom, bearing, pitch) =
            get_view_params(&lats, &lons);
    }
    let mut trace = if !records.is_empty() {
        ScatterMapbox::new(lats, lons)
    } else {
        // If we don't have a data point, mapbox-gl-js complains about "there is
        // already a source with this ID"
        ScatterMapbox::new(vec![0.], vec![0.])
    };

    trace = trace
        .marker(marker)
        .hover_text_array(get_hovertext(&records));

    let layout = Layout::new()
        // transparent paper: no white flashes on plot load
        .paper_background_color(Rgba::new(0, 0, 0, 0.))
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(mapbox_style)
                .center(Center::new(lat_center, lon_center))
                .zoom(zoom)
                .bearing(bearing)
                .pitch(pitch),
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

/// Get the hovertext to show when the datapoints are clicked.
fn get_hovertext(records: &[&common::Location]) -> Vec<String> {
    let local_offset = time::UtcOffset::current_local_offset().unwrap();
    records
        .iter()
        .map(|loc| {
            format!(
                "+/-{:.2} m, {:.2} m/s, {:.2}°<br>{}",
                loc.accuracy,
                loc.speed,
                loc.course,
                loc.datetime
                    .to_offset(local_offset)
                    .format(&time::format_description::well_known::Rfc2822)
                    .unwrap()
            )
        })
        .collect()
}

pub fn map_colorbar() -> plotly::common::ColorBar {
    plotly::common::ColorBar::new()
        .background_color(plotly::color::NamedColor::Black)
        .orientation(plotly::common::Orientation::Horizontal)
        .thickness(20)
        .ticks(plotly::common::Ticks::Inside)
        .x_pad(50.)
        .y(0.)
}

/// Generate an empty plot layout without traces
#[allow(dead_code)]
pub fn empty_plot(marker: Marker) -> plotly::Plot {
    // If we don't have a data point, mapbox-gl-js complains about "there is
    // already a source with this ID"
    let trace = ScatterMapbox::new(vec![0.0], vec![0.0]).marker(marker);
    let layout = Layout::new()
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::Named(NamedMapboxStyle::StamenTerrain))
                .zoom(1),
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

/// A wrapper around Vec<f64> which implements the plotly Color trait
#[derive(Clone, Debug, serde::Serialize)]
pub struct Colorvec(pub Vec<f64>);
impl plotly::common::color::Color for Colorvec {}

/// Floats don't implement Ord, so we have to do this
fn float_min(vals: &[f64]) -> f64 {
    *vals
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}
fn float_max(vals: &[f64]) -> f64 {
    *vals
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}

/// Recenter a map. Make sure there is at least one data point in the vectors
/// when calling this.
fn get_view_params(lats: &[f64], lons: &[f64]) -> (f64, f64, u8, f64, f64) {
    if lats.is_empty() {
        return (0., 0., 0, 0., 0.);
    }
    let lat_center = (float_max(lats) + float_min(lats)) / 2.;
    let lon_center = (float_max(lons) + float_min(lons)) / 2.;
    let lat_range = float_max(lats) - float_min(lats);
    let lon_range = float_max(lons) - float_min(lons);
    let lat_zoom = (360. / lat_range * lat_center.to_radians().cos()).log2();
    let lon_zoom = (360. / lon_range).log2();
    let zoom = if lat_zoom > lon_zoom {
        lon_zoom
    } else {
        lat_zoom
    };
    let mut zoom = zoom.floor() - 1.;
    zoom = zoom.clamp(0., 16.);
    let zoom = zoom as u8;
    (lat_center, lon_center, zoom, 0., 0.)
}
