use std::{cell::RefCell, rc::Rc};

use serde_json::json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Map;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Map)]
    pub fn new(options: &JsValue) -> Map;

    #[wasm_bindgen(method)]
    pub fn on(this: &Map, event: &str, listener: &JsValue);

    #[wasm_bindgen(method, js_name = addSource)]
    pub fn add_source(this: &Map, id: &str, data: &JsValue);

    #[wasm_bindgen(method, js_name = addLayer)]
    pub fn add_layer(this: &Map, data: &JsValue);
}

pub fn map_plot(
    plot_id: &str,
    records: &Vec<&common::Location>,
    basemap: String,
) {
    let lats: Vec<_> = records.iter().map(|x| x.lat).collect();
    let lons: Vec<_> = records.iter().map(|x| x.lon).collect();
    let (lat_center, lon_center, zoom, _, _) = get_view_params(&lats, &lons);

    // Create the map and start it loading
    let mut opts = json!({
        "container": plot_id,
        // "style": "https://demotiles.maplibre.org/style.json",
        "style": basemap,
        // "center": [-122, 37.5],
        "center": [lon_center, lat_center],
        "zoom": zoom,
    });
    let jsv = val_to_jsval(&opts);
    let map = Map::new(&jsv);
    // Wrap the map in a Rc<RefCell<>> so we can clone references to it and pass
    // those into closures.
    let map = Rc::new(RefCell::new(map));

    let geojson = get_geojson_features(records);
    let newlayer = json!({
        "id": "conferences",
        "type": "circle",
        "source": "conferences",
        "paint": {
            "circle-radius": 5,
            "circle-color": "#3887be",
        }
    });

    let on_load: Box<dyn FnMut()> = {
        let map = map.clone();
        Box::new(move || {
            map.borrow_mut().add_source(
                "conferences",
                &val_to_jsval(&json!({
                    "type": "geojson",
                    "data": geojson,
                })),
            );
            map.borrow_mut().add_layer(&val_to_jsval(&newlayer));
        })
    };

    map.borrow_mut()
        .on("load", &Closure::wrap(on_load).into_js_value());
}

/// Convert from a serde_json::Value (loosely-typed object) to a json JsValue
/// owned by javascript.
fn val_to_jsval(v: &serde_json::Value) -> JsValue {
    js_sys::JSON::parse(&serde_json::to_string(v).unwrap())
        .expect("Invalid JSON")
}

/// Turn the vector of records into a geojson object
fn get_geojson_features(records: &Vec<&common::Location>) -> serde_json::Value {
    let arr: Vec<_> = records
        .iter()
        .map(|x| {
            json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [x.lon, x.lat]
                },
            })
        })
        .collect();
    json!({
        "type": "FeatureCollection",
        "features": arr,
    })
}

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
