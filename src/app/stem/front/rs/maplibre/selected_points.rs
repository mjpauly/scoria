use std::rc::Rc;

use common::{map_style::MapStyle, mounted::MountID, LngLat};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use crate::maplibre::val_to_jsval;

use super::binds;

pub const SOURCE_ID: &str = "selected_points";
pub const LAYER_ID: &str = "selected_points";

/// Selections are a while halo around a point that is behind the points layers
/// in z height.
pub fn make_layer(map_style: &MapStyle) -> Value {
    json!({
        "id": LAYER_ID,
        "type": "circle",
        "source": SOURCE_ID,
        "paint": {
            "circle-radius": map_style.marker_size,
            "circle-color": "#ffffff",
            "circle-opacity": 0.0,
            // white halo
            "circle-stroke-width": 4,
            "circle-stroke-color": if map_style.basemap_style.is_dark() {
                "#dddddd"
            } else {
                "#222222"
            },
            "circle-stroke-opacity": 1.0,
        }
    })
}

pub fn update_selected(
    map: &Rc<binds::Map>,
    selected: &[((MountID, time::OffsetDateTime), LngLat)],
) -> Result<(), JsValue> {
    map.get_source(SOURCE_ID)
        .set_data(&val_to_jsval(&make_selected_geojson(selected)));
    Ok(())
}

pub fn update_after_restyle(
    map: Rc<binds::Map>,
    selected: Vec<((MountID, time::OffsetDateTime), LngLat)>,
) {
    // once_into_js frees the closure after its single invocation
    map.clone().once(
        "styledata",
        &Closure::once_into_js(move || {
            update_selected(&map, &selected).unwrap();
        }),
    );
}

/// Make the selected points geojson source
fn make_selected_geojson(
    selected: &[((MountID, time::OffsetDateTime), LngLat)],
) -> Value {
    let features = selected
        .iter()
        .map(|p| {
            json!({
                "type": "Feature",
                "geometry": {
                     "type": "Point",
                     "coordinates": [ p.1.lng, p.1.lat ]
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "FeatureCollection",
        "features": features,
    })
}
