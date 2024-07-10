use std::rc::Rc;

use common::LngLat;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use crate::maplibre::val_to_jsval;

use super::binds;

pub const SOURCE_ID: &str = "selected_points";
pub const LAYER_ID: &str = "selected_points";

/// Selections are a while halo around a point that is behind the points layers
/// in z height.
pub fn make_layer(marker_size: usize) -> Value {
    json!({
        "id": LAYER_ID,
        "type": "circle",
        "source": SOURCE_ID,
        "paint": {
            "circle-radius": marker_size,
            "circle-color": "#ffffff",
            "circle-opacity": 0.0,
            // white halo
            "circle-stroke-width": 4,
            "circle-stroke-color": "#ffffff",
            "circle-stroke-opacity": 0.8,
        }
    })
}

pub fn update_selected(
    map: &Rc<binds::Map>,
    selected: &[(time::OffsetDateTime, LngLat)],
) -> Result<(), JsValue> {
    map.get_source(SOURCE_ID)
        .set_data(&val_to_jsval(&make_selected_geojson(selected)));
    Ok(())
}

/// Make the selected points geojson source
fn make_selected_geojson(selected: &[(time::OffsetDateTime, LngLat)]) -> Value {
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
