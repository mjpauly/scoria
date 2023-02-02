//! Generate visualizations using plotly, outputted to [docdir]/foo.html

use plotly::{
    common::{color, Marker},
    configuration::{Configuration, DisplayModeBar},
    layout::{Center, Mapbox, MapboxStyle, Margin},
    Layout, Plot, ScatterMapbox,
};

use crate::database;
use crate::paths;

/// Generate a visualization of the past week
pub async fn gen_past_week_viz() {
    let out_path = paths::get_viz_path();
    let records = database::get_records_past_week().await;

    // filter out records where accuracy is worse (larger) than 20m
    let records = records
        .iter()
        .filter(|x| x.accuracy < 20.0)
        .collect::<Vec<&database::Location>>();
    let lats: Vec<f64> = records.iter().map(|x| x.lat).collect();
    let lons: Vec<f64> = records.iter().map(|x| x.lon).collect();

    // calculate where to put the center
    let mean_lat = lats.iter().sum::<f64>() / lats.len() as f64;
    let mean_lon = lons.iter().sum::<f64>() / lons.len() as f64;

    let trace = ScatterMapbox::new(lats, lons).marker(
        Marker::new()
            .opacity(0.8)
            .color(color::NamedColor::OrangeRed),
    );
    let layout = Layout::new()
        .margin(Margin::new().top(50).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::StamenTerrain)
                .center(Center::new(mean_lat, mean_lon))
                .zoom(8),
        );
    let config = Configuration::new()
        .responsive(true)
        .fill_frame(true)
        .display_logo(false)
        .display_mode_bar(DisplayModeBar::False);

    let mut plot = Plot::new();
    plot.use_local_plotly();
    plot.add_trace(trace);
    plot.set_layout(layout);
    plot.set_configuration(config);

    plot.write_html(out_path);
}

/// Generate a visualization of the past week
pub async fn gen_viz(
    start_epoch: i64,
    end_epoch: i64,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) {
    let out_path = paths::get_viz_path();
    let records =
        database::get_records_time_range(start_epoch, end_epoch).await;

    // filter out records where accuracy is worse (larger) than 20m
    let records = records
        .iter()
        .filter(|x| x.accuracy < 20.0)
        .collect::<Vec<&database::Location>>();
    let lats: Vec<f64> = records.iter().map(|x| x.lat).collect();
    let lons: Vec<f64> = records.iter().map(|x| x.lon).collect();

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
        .margin(Margin::new().top(50).left(0).bottom(0).right(0))
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::StamenTerrain)
                .center(Center::new(mean_lat, mean_lon))
                .zoom(8),
        );
    let config = Configuration::new()
        .responsive(true)
        .fill_frame(true)
        .display_logo(false)
        .display_mode_bar(DisplayModeBar::False);

    let mut plot = Plot::new();
    plot.use_local_plotly();
    plot.add_trace(trace);
    plot.set_layout(layout);
    plot.set_configuration(config);

    plot.write_html(out_path);
}

#[cfg(test)]
mod tests {
    use sqlx::types::time;

    use crate::database;
    use crate::tests::test_setup;

    use super::*;

    async fn viz_test_setup(test_dir: &str) {
        if std::fs::metadata(test_dir).is_ok() {
            // need to clear it manually if left over from previous test
            std::fs::remove_dir_all(test_dir).unwrap();
        }
        test_setup(test_dir).await;
    }

    async fn log_test_data(now: i64) {
        use database::log_location;
        log_location(37.59, -122.09, 0.0, 0.0, 0.0, now)
            .await
            .unwrap();
        log_location(37.6, -122.09, 0.0, 0.0, 0.0, now - 10)
            .await
            .unwrap();
        log_location(37.6, -122.1, 0.0, 0.0, 0.0, now)
            .await
            .unwrap();
        log_location(37.59, -122.1, 0.0, 0.0, 0.0, now - 10)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_plot() {
        // can view outputs in /tmp directory since writing there is allowed
        // from the macos sandbox
        viz_test_setup("/tmp/test_plot/").await;

        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        log_test_data(now).await;
        gen_past_week_viz().await;
    }

    #[tokio::test]
    async fn test_custom_plot() {
        viz_test_setup("/tmp/test_custom_plot/").await;

        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        log_test_data(now).await;
        gen_viz(now - 5, now, 0.9, 0.05, 0.5, 0.5).await;
    }
}
