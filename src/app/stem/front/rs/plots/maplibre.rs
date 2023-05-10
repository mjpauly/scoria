use std::rc::Rc;

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use crate::components::map_styler::ColoredDataStream;
use crate::float;
use crate::plots::cmaps;

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, PartialEq)]
    pub type Map;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Map)]
    pub fn new(options: &JsValue) -> Map;

    #[wasm_bindgen(method)]
    pub fn on(this: &Map, event: &str, listener: &JsValue);
    #[wasm_bindgen(method)]
    pub fn once(this: &Map, event: &str, listener: &JsValue);

    #[wasm_bindgen(method, js_name = addSource)]
    pub fn add_source(this: &Map, id: &str, data: &JsValue);
    #[wasm_bindgen(method, js_name = getSource)]
    pub fn get_source(this: &Map, id: &str) -> Source;

    #[wasm_bindgen(method, js_name = addLayer)]
    pub fn add_layer(this: &Map, data: &JsValue);
    #[wasm_bindgen(method, js_name = removeLayer)]
    pub fn remove_layer(this: &Map, id: &str);

    #[wasm_bindgen(method, js_name = setStyle)]
    pub fn set_style(this: &Map, style: &str);

    #[wasm_bindgen(method, js_name = addControl)]
    pub fn add_navigation_control(
        this: &Map,
        control: NavigationControl,
        position: &str,
    );

    pub type Source;

    #[wasm_bindgen(method, js_name = setData)]
    pub fn set_data(this: &Source, data: &JsValue) -> JsValue;

    pub type NavigationControl;

    #[wasm_bindgen(constructor, js_namespace = maplibregl,
                   js_name = NavigationControl)]
    pub fn new(options: &JsValue) -> NavigationControl;

}

static SOURCE_ID: &str = "datapoints";
static LAYER_ID: &str = "datapoints";

// TODO: colorbar, popup on select, re-center button
pub fn new_map(
    plot_id: &str,
    records: &[&common::Location],
    basemap: &str,
    marker_size: usize,
    marker_color: &str,
    marker_opacity: f64,
    colored_datastream: &ColoredDataStream,
    callback: Box<dyn Fn()>, // closure to run in the maps on_load handler
) -> Rc<Map> {
    let lats: Vec<_> = records.iter().map(|x| x.lat).collect();
    let lons: Vec<_> = records.iter().map(|x| x.lon).collect();
    let (lat_center, lon_center, zoom, _, _) = get_view_params(&lats, &lons);

    // Create the map and start it loading
    let opts = json!({
        "container": plot_id,
        "style": basemap,
        "center": [lon_center, lat_center],
        "zoom": zoom,
    });
    let map = Map::new(&val_to_jsval(&opts));

    // Add compass control
    map.add_navigation_control(
        NavigationControl::new(&val_to_jsval(&json!({
            "showCompass": true,
            "showZoom": false,
            "visualizePitch": true,
        }))),
        "bottom-left",
    );

    // === Add data source and visible layer ===

    let geojson = make_geojson(records, colored_datastream);
    let newsource = json!({
        "type": "geojson",
        "data": geojson,
    });
    let newlayer = make_layer(
        marker_size,
        marker_color,
        marker_opacity,
        colored_datastream,
    );

    // Wrap the map in a Rc so we can clone references to it and pass those into
    // closures and share the map reference.
    let map = Rc::new(map);

    let on_load: Box<dyn FnMut()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(SOURCE_ID, &val_to_jsval(&newsource));
            map.add_layer(&val_to_jsval(&newlayer));
            callback();
        })
    };

    map.on("load", &Closure::wrap(on_load).into_js_value());
    map
}

/// Update the map's data source
pub fn update_data(
    map: Rc<Map>,
    records: &[&common::Location],
    colored_datastream: &ColoredDataStream,
) {
    let geojson = make_geojson(records, colored_datastream);
    map.get_source(LAYER_ID).set_data(&val_to_jsval(&geojson));
}

/// Restyle the layer by removing the old layer and adding it back
#[allow(dead_code)]
pub fn restyle_layer(
    map: Rc<Map>,
    marker_size: usize,
    marker_color: &str,
    marker_opacity: f64,
    colored_datastream: &ColoredDataStream,
) {
    let newlayer = make_layer(
        marker_size,
        marker_color,
        marker_opacity,
        colored_datastream,
    );
    map.remove_layer(LAYER_ID);
    map.add_layer(&val_to_jsval(&newlayer));
}

/// Restyles the whole plot. Necessary if changing the basemap layer since
/// sources and layers are removed.
pub fn restyle(
    map: Rc<Map>,
    records: &[&common::Location],
    basemap: &str,
    marker_size: usize,
    marker_color: &str,
    marker_opacity: f64,
    colored_datastream: &ColoredDataStream,
    callback: Box<dyn Fn()>, // closure to run when restyling is complete
) {
    let geojson = make_geojson(records, colored_datastream);
    let newsource = json!({
        "type": "geojson",
        "data": geojson,
    });
    let newlayer = make_layer(
        marker_size,
        marker_color,
        marker_opacity,
        colored_datastream,
    );

    // restyling the basemap also removes our sources and layers, so we need to
    // add them back when the "styledata" event is emitted.
    let on_load: Box<dyn FnMut()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(SOURCE_ID, &val_to_jsval(&newsource));
            map.add_layer(&val_to_jsval(&newlayer));
            callback();
        })
    };

    map.once("styledata", &Closure::wrap(on_load).into_js_value());
    map.set_style(basemap);
}

/// Convert from a serde_json::Value (loosely-typed object) to a json JsValue
/// owned by javascript.
fn val_to_jsval(v: &Value) -> JsValue {
    js_sys::JSON::parse(&serde_json::to_string(v).unwrap())
        .expect("Invalid JSON")
}

#[allow(dead_code)]
fn jsval_to_val(v: JsValue) -> Value {
    // serde_wasm_bindgen::from_value(v).unwrap()
    let s: String = js_sys::JSON::stringify(&v).unwrap().into();
    s.into()
}

fn make_layer(
    marker_size: usize,
    marker_color: &str,
    marker_opacity: f64,
    colored_datastream: &ColoredDataStream,
) -> Value {
    json!({
        "id": LAYER_ID,
        "type": "circle",
        "source": SOURCE_ID,
        "paint": {
            "circle-radius": marker_size,
            "circle-color":
                if colored_datastream.is_some() {
                    json!(["get", "color"])
                } else {
                    json!(marker_color)
                },
            "circle-opacity": marker_opacity,
        }
    })
}

/// Turn the vector of records into a geojson object
fn make_geojson(
    records: &[&common::Location],
    colored_datastream: &ColoredDataStream,
) -> Value {
    let local_offset = time::UtcOffset::current_local_offset().unwrap();
    let hovertext = move |loc: &common::Location| {
        format!(
            "({:.8}°, {:.8}°)\
            <br>+/-{:.2} m, {:.2} m/s, {:.2}°\
            <br>{}",
            loc.lat,
            loc.lon,
            loc.accuracy,
            loc.speed,
            loc.course,
            loc.datetime
                .to_offset(local_offset)
                .format(&time::format_description::well_known::Rfc2822)
                .unwrap()
        )
    };
    let arr: Vec<_> = if colored_datastream.is_some() {
        let (cmin, cmax, cmap_to_use) =
            colored_datastream.get_cmap_params(records);
        records
            .iter()
            .map(|x| {
                let val = colored_datastream.get_stream(x);
                let color = cmaps::get_data_color(cmap_to_use, val, cmin, cmax);
                // select light or dark text color so it shows up against the
                // popup background
                let textcolor = if (val - cmin) / (cmax - cmin) > 0.5 {
                    "#eee"
                } else {
                    "#111"
                };
                json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [x.lon, x.lat]
                    },
                    "properties": {
                        "hovertext": hovertext(x),
                        "color": color,
                        "textcolor": textcolor,
                    },
                })
            })
            .collect()
    } else {
        records
            .iter()
            .map(|x| {
                json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [x.lon, x.lat]
                    },
                    "properties": {
                        "hovertext": hovertext(x),
                    },
                })
            })
            .collect()
    };
    json!({
        "type": "FeatureCollection",
        "features": arr,
    })
}

/// Calculate the center of a map. Does not take the map size into account, so
/// is overly conservative (zooms further out than needed)
fn get_view_params(lats: &[f64], lons: &[f64]) -> (f64, f64, u8, f64, f64) {
    if lats.is_empty() {
        return (0., 0., 0, 0., 0.);
    }
    let lat_center = (float::max(lats) + float::min(lats)) / 2.;
    let lon_center = (float::max(lons) + float::min(lons)) / 2.;
    let lat_range = float::max(lats) - float::min(lats);
    let lon_range = float::max(lons) - float::min(lons);
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
