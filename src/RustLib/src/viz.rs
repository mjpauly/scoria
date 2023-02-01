//! Generate visualizations using plotly, outputted to [docdir]/foo.html

use plotly::{
    common::Marker,
    layout::{Center, DragMode, Mapbox, MapboxStyle, Margin},
    Layout, Plot, ScatterMapbox,
};

use crate::database;
use crate::paths;

/// Generate a visualization of the past week
pub async fn gen_past_week_viz() {
    let out_path = std::path::Path::new(&paths::get_storage_dir().unwrap())
        .join("week.html");
    // let binding = runtime::get_runtime_binding();
    // let rt = binding.borrow();
    // let records =
    // rt.block_on(async { database::get_records_past_week().await });
    let records = database::get_records_past_week().await;

    // filter out records where accuracy is worse (larger) than 20m
    let records = records
        .iter()
        .filter(|x| x.accuracy < 20.0)
        .collect::<Vec<&database::Location>>();
    let lats = records.iter().map(|x| x.lat).collect();
    let lons = records.iter().map(|x| x.lon).collect();

    let trace = ScatterMapbox::new(lats, lons).marker(Marker::new());

    let layout = Layout::new()
        .margin(Margin::new().top(0).left(0).bottom(0).right(0))
        .mapbox(Mapbox::new().style(MapboxStyle::OpenStreetMap));

    let mut plot = Plot::new();
    plot.add_trace(trace);
    plot.set_layout(layout);
    plot.write_html(out_path);

    // testing: get output html for viewing

    // plot.write_html(out_path.clone());
    // std::fs::copy(out_path, "/tmp/week.html").unwrap();
}

#[cfg(test)]
mod tests {
    use sqlx::types::time;

    use crate::database;
    use crate::paths;

    use super::*;

    #[tokio::test]
    async fn test_plot() {
        let storage_dir = String::from("test_plot/");
        std::fs::create_dir(&storage_dir).unwrap();
        paths::set_storage_dir(storage_dir);
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        database::log_location(0.0, 0.0, 0.0, 0.0, 0.0, now).await;
        database::log_location(1.0, 0.0, 0.0, 0.0, 0.0, now).await;
        database::log_location(1.0, 1.0, 0.0, 0.0, 0.0, now).await;
        database::log_location(0.0, 1.0, 0.0, 0.0, 0.0, now).await;
        gen_past_week_viz().await;
    }
}
