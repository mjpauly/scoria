//! Bindings and data processing for the Maplibre charts.
//!
//! TODO: maplibre just stringifies the json sources anyways. Just do this in
//! backend and link to the geojson source?
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use serde_json::{json, Value};
use std::time::Duration;
use wasm_bindgen::{prelude::*, JsCast};

use crate::components::map_styler::get_cmap_params;
use crate::float;
use crate::plots::cmaps;
use common::map_style::{ColoredDataStream, Rgba};
use common::view_position::ViewPosition;

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, PartialEq)]
    pub type Map;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Map)]
    pub fn new(options: &JsValue) -> Map;

    #[wasm_bindgen(method)]
    pub fn on(this: &Map, event: &str, listener: &JsValue);
    #[wasm_bindgen(method, js_name = on)]
    pub fn on_layer(this: &Map, event: &str, layer: &str, listener: &JsValue);
    #[wasm_bindgen(method)]
    pub fn once(this: &Map, event: &str, listener: &JsValue);

    #[wasm_bindgen(method, js_name = addSource)]
    pub fn add_source(this: &Map, id: &str, data: &JsValue);
    #[wasm_bindgen(method, js_name = getSource)]
    pub fn get_source(this: &Map, id: &str) -> Source;

    #[wasm_bindgen(method, js_name = addLayer)]
    pub fn add_layer(this: &Map, data: &JsValue);
    #[wasm_bindgen(method, js_name = addLayer)]
    pub fn add_layer_below(this: &Map, data: &JsValue, below_id: &JsValue);
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

    #[wasm_bindgen(method, js_name = getCenter)]
    pub fn get_center(this: &Map) -> LngLat;
    #[wasm_bindgen(method, js_name = getZoom)]
    pub fn get_zoom(this: &Map) -> f64;
    #[wasm_bindgen(method, js_name = getBearing)]
    pub fn get_bearing(this: &Map) -> f64;
    #[wasm_bindgen(method, js_name = getPitch)]
    pub fn get_pitch(this: &Map) -> f64;

    #[wasm_bindgen(method, js_name = flyTo)]
    pub fn fly_to(this: &Map, options: &JsValue);
    #[wasm_bindgen(method, js_name = easeTo)]
    pub fn ease_to(this: &Map, options: &JsValue);

    pub type Source;

    #[wasm_bindgen(method, js_name = setData)]
    pub fn set_data(this: &Source, data: &JsValue) -> JsValue;

    pub type NavigationControl;

    #[wasm_bindgen(constructor, js_namespace = maplibregl,
                   js_name = NavigationControl)]
    pub fn new(options: &JsValue) -> NavigationControl;

    pub type LngLat;

    #[wasm_bindgen(method, getter)]
    pub fn lng(this: &LngLat) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn lat(this: &LngLat) -> f64;

    pub type Popup;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Popup)]
    pub fn new(options: &JsValue) -> Popup;
    #[wasm_bindgen(method, js_name = setLngLat)]
    pub fn set_lng_lat(this: &Popup, coordinates: &JsValue);
    #[wasm_bindgen(method, js_name = setHTML)]
    pub fn set_html(this: &Popup, description: &JsValue);
    #[wasm_bindgen(method, js_name = addTo)]
    pub fn add_to(this: &Popup, map: &Map);
}

static POINTS_SOURCE_ID: &str = "points";
static POINTS_LAYER_ID: &str = "points";
static LINES_SOURCE_ID: &str = "lines";
static LINES_LAYER_ID: &str = "lines";

pub fn view_pos_from_map(map: &Map) -> ViewPosition {
    let center = map.get_center();
    ViewPosition {
        lng: center.lng(),
        lat: center.lat(),
        zoom: map.get_zoom(),
        bearing: map.get_bearing(),
        pitch: map.get_pitch(),
    }
}

// TODO: popup on select
#[allow(clippy::too_many_arguments)]
pub fn new_map(
    plot_id: &str,
    basemap: &str,
    marker_size: usize,
    line_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
    view_position: &ViewPosition,
    on_load_callback: Box<dyn Fn()>, // closure to run when the plot loads
    // closure to run with pan/zoom data
    on_view_change_callback: Box<dyn Fn(ViewPosition)>,
    // closure to get popup text and background color given the
    // data point's lng, lat position and color property, if it exists
    get_popup_text: impl Fn(f64, f64, Option<String>) -> ((f64, f64), String, String)
        + Clone
        + 'static,
) -> Rc<Map> {
    // Create the map and start it loading
    let opts = json!({
        "container": plot_id,
        "style": basemap,
        "center": [view_position.lng, view_position.lat],
        "zoom": view_position.zoom,
        "bearing": view_position.bearing,
        "pitch": view_position.pitch,
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

    // create a geojson source without features
    let geojson = create_geojson(&Array::new());
    let source = create_source(&geojson);
    let points_layer =
        make_points_layer(marker_size, marker_color, colored_datastream);
    let lines_layer =
        make_lines_layer(line_size, marker_color, colored_datastream);

    // Wrap the map in a Rc so we can clone references to it and pass those into
    // closures and share the map reference.
    let map = Rc::new(map);

    let popup_callback = get_popup_callback(map.clone(), get_popup_text);

    // Add the data source and layer and notify the yew component when loading
    // of the map has finished.
    let on_load: Box<dyn Fn()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(POINTS_SOURCE_ID, &source);
            map.add_source(LINES_SOURCE_ID, &source);
            map.add_layer(&val_to_jsval(&points_layer));
            map.add_layer_below(
                &val_to_jsval(&lines_layer),
                &POINTS_LAYER_ID.into(),
            );
            let popup_callback = popup_callback.clone();
            map.on_layer(
                "click",
                POINTS_LAYER_ID,
                &Closure::wrap(
                    Box::new(popup_callback) as Box<dyn Fn(&JsValue)>
                )
                .into_js_value(),
            );
            on_load_callback();
        })
    };
    map.on("load", &Closure::wrap(on_load).into_js_value());

    // Notify the yew component of the new view position whenever it changes.
    let on_view_change: Box<dyn Fn()> = {
        let map = map.clone();
        Box::new(move || on_view_change_callback(view_pos_from_map(&map)))
    };
    map.on("moveend", &Closure::wrap(on_view_change).into_js_value());

    map
}

fn get_popup_callback(
    map: Rc<Map>,
    get_popup_text: impl Fn(f64, f64, Option<String>) -> ((f64, f64), String, String)
        + Clone
        + 'static,
) -> impl Fn(&JsValue) + Clone {
    move |event: &JsValue| {
        let features = Reflect::get(event, &"features".into()).unwrap();
        let first = features.dyn_ref::<Array>().unwrap().at(0);

        // get the color of the data point
        let props = Reflect::get(&first, &"properties".into()).unwrap();
        let color_obj = Reflect::get(&props, &"color".into()).unwrap();
        let color = if !color_obj.is_undefined() {
            Some(color_obj.as_string().unwrap())
        } else {
            None
        };

        // get the lng, lat of the data point so we can find it
        let geometry = Reflect::get(&first, &"geometry".into()).unwrap();
        let coordinates =
            Reflect::get(&geometry, &"coordinates".into()).unwrap();
        let coord_arr = coordinates.dyn_ref::<Array>().unwrap();
        let lng = coord_arr.at(0).as_f64().unwrap();
        let lat = coord_arr.at(1).as_f64().unwrap();

        let ((lng, lat), text, bg_color) = get_popup_text(lng, lat, color);
        // place popup at exact coordinates of the point
        let newcoords = Array::new();
        newcoords.set(0, lng.into());
        newcoords.set(1, lat.into());

        let text_color = if cmaps::hex_color_is_bright(&bg_color) {
            "#000000" // dark text
        } else {
            "#ffffff"
        };

        // add the css rules for coloring the popup
        style_popup(&bg_color, text_color);

        let opts = json!({
            "closeButton": false,
        });
        let popup = Popup::new(&val_to_jsval(&opts));
        popup.set_lng_lat(&newcoords);
        popup.set_html(&text.into());
        popup.add_to(&map);
    }
}

/// Style the maplibre popup with the given background color and text color
// TODO: delete old css rules
fn style_popup(bg_color: &str, text_color: &str) {
    let content_css = format!(
        "background-color: {bg_color}; \
        color: {text_color}; \
        padding: 2px 5px; \
        opacity: 0.9;"
    );
    add_css_rule(".maplibregl-popup-content", &content_css);

    // for the tip we need to add 4 rules, one for each popup orientation
    let orientations = [
        ("top", "bottom"), // (anchor-{orientation}, border-{orientation})
        ("bottom", "top"),
        ("right", "left"),
        ("left", "right"),
        ("top-left", "bottom"),
        ("top-right", "bottom"),
        ("bottom-left", "top"),
        ("bottom-right", "top"),
    ];
    for (class_orientation, border_orientation) in orientations {
        let tip_class = format!(
            ".maplibregl-popup-anchor-{class_orientation} \
            .maplibregl-popup-tip"
        );
        let tip_css = format!(
            "border-{border_orientation}-color: {bg_color}; \
            opacity: 0.9;"
        );
        add_css_rule(&tip_class, &tip_css);
    }
}

/// Add a css rules to the document given the class name and the rules to add.
/// The rule is placed at the end of the last stylesheet, allowing for
/// overriding earlier class definitions without using !important.
pub fn add_css_rule(class_name: &str, rule: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let style_sheets = document.style_sheets();
    let last_sheet_index = style_sheets.length();
    let style_sheet = style_sheets
        .get(last_sheet_index - 1)
        .expect("Failed to get the last stylesheet");

    if let Ok(css_style_sheet) =
        style_sheet.dyn_into::<web_sys::CssStyleSheet>()
    {
        let rule_text = format!("{} {{ {} }}", class_name, rule);
        let insert_index = css_style_sheet.css_rules().unwrap().length();

        css_style_sheet
            .insert_rule_with_index(&rule_text, insert_index)
            .expect("Failed to insert CSS rule");
    }
}

/// Update the map's data source
pub async fn update_data(
    map: Rc<Map>,
    records: &[&common::Location],
    marker_size: usize,
    line_size: usize,
    colored_datastream: &ColoredDataStream,
) {
    let (points_geojson, lines_geojson) =
        make_geojson_async(records, marker_size, line_size, colored_datastream)
            .await;
    map.get_source(POINTS_SOURCE_ID).set_data(&points_geojson);
    map.get_source(LINES_SOURCE_ID).set_data(&lines_geojson);
}

/// Restyle the layer by removing the old layer and adding it back
#[allow(dead_code)]
pub fn restyle_layer(
    map: Rc<Map>,
    marker_size: usize,
    line_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) {
    let points_layer =
        make_points_layer(marker_size, marker_color, colored_datastream);
    let lines_layer =
        make_lines_layer(line_size, marker_color, colored_datastream);
    map.remove_layer(POINTS_LAYER_ID);
    map.remove_layer(LINES_LAYER_ID);
    map.add_layer(&val_to_jsval(&points_layer));
    map.add_layer_below(&val_to_jsval(&lines_layer), &POINTS_LAYER_ID.into());
}

/// Restyles the whole plot. Necessary if changing the basemap layer since
/// sources and layers are removed.
#[allow(clippy::too_many_arguments)]
pub async fn restyle(
    map: Rc<Map>,
    records: &[&common::Location],
    basemap: &str,
    marker_size: usize,
    line_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
    callback: Box<dyn Fn()>, // closure to run when restyling is complete
) {
    let (points_geojson, lines_geojson) =
        make_geojson_async(records, marker_size, line_size, colored_datastream)
            .await;
    let points_source = create_source(&points_geojson);
    let lines_source = create_source(&lines_geojson);
    let points_layer =
        make_points_layer(marker_size, marker_color, colored_datastream);
    let lines_layer =
        make_lines_layer(line_size, marker_color, colored_datastream);

    // restyling the basemap also removes our sources and layers, so we need to
    // add them back when the "styledata" event is emitted.
    let on_load: Box<dyn FnMut()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(POINTS_SOURCE_ID, &points_source);
            map.add_source(LINES_SOURCE_ID, &lines_source);
            map.add_layer(&val_to_jsval(&points_layer));
            map.add_layer_below(
                &val_to_jsval(&lines_layer),
                &POINTS_LAYER_ID.into(),
            );
            callback();
        })
    };

    map.once("styledata", &Closure::wrap(on_load).into_js_value());
    map.set_style(basemap);
}

fn create_source(geojson: &Object) -> Object {
    let source = Object::new();
    Reflect::set(&source, &"type".into(), &"geojson".into()).unwrap();
    Reflect::set(&source, &"data".into(), geojson).unwrap();
    source
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

fn make_points_layer(
    marker_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) -> Value {
    json!({
        "id": POINTS_LAYER_ID,
        "type": "circle",
        "source": POINTS_SOURCE_ID,
        "paint": {
            "circle-radius": marker_size,
            "circle-color":
                if colored_datastream.is_some() {
                    json!(["get", "color"])
                } else {
                    json!(marker_color.rgb)
                },
            "circle-opacity": marker_color.a,
            // An invisible stroke of 5px to makes the data points easier to
            // click.
            "circle-stroke-width": 5,
            "circle-stroke-color": "#ffffff",
            "circle-stroke-opacity": 0.,
        }
    })
}

fn make_lines_layer(
    line_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) -> Value {
    json!({
        "id": LINES_LAYER_ID,
        "type": "line",
        "source": LINES_SOURCE_ID,
        "paint": {
            "line-width": line_size,
            "line-color":
                if colored_datastream.is_some() {
                    json!(["get", "color"])
                } else {
                    json!(marker_color.rgb)
                },
            "line-opacity": marker_color.a,
        }
    })
}

/// Sleep for a very short duration to yield execution back to the scheduler,
/// and keep from blocking the UI
pub async fn async_yield() {
    // const for compile-time evaluation
    const SLEEP_DURATION: Duration = Duration::from_micros(1);
    yew::platform::time::sleep(SLEEP_DURATION).await;
}

/// Turn the vector of records into two geojson objects, one for the data points
/// and one for the lines
// TODO: split lines in two at antimeridian
async fn make_geojson_async(
    records: &[&common::Location],
    marker_size: usize,
    line_size: usize,
    colored_datastream: &ColoredDataStream,
) -> (Object, Object) {
    // log::debug!("make_geojson_async processing {} records", records.len());
    // yield before computing cmap params
    async_yield().await;
    let cmap_params = get_cmap_params(colored_datastream, records);

    // create the keys to properties once
    let keys = ObjKeys {
        obj_type: JsValue::from_str("type"),
        feature: JsValue::from_str("Feature"),
        point: JsValue::from_str("Point"),
        line: JsValue::from_str("LineString"),
        coordinates: JsValue::from_str("coordinates"),
        geometry: JsValue::from_str("geometry"),
        color: JsValue::from_str("color"),
        properties: JsValue::from_str("properties"),
    };

    let point_features = Array::new();
    let line_features = Array::new();

    // Yield at the greater of every 1000 iterations or the total number / 100.
    // We let it increase so as to reduce overhead from the timers at the cost
    // of jank if the number of data points is very large.
    let yield_period = 10_000.max(records.len() / 100);
    for i in 0..records.len() {
        if i % yield_period == 0 {
            // periodically yield back execution so we don't block too long
            async_yield().await;
        }

        // Whether or not to make the point feature
        let make_point = marker_size > 0;
        // With the lines we index one ahead to get the lind endpoint, so we
        // don't want to make the line on the final record
        let make_line = line_size > 0 && i < records.len() - 1;

        let coords = create_coordinates(records[i]);

        let mut point_feature = None;
        let mut line_feature = None;
        if make_point {
            let point_geometry = create_geometry(&keys, &keys.point, &coords);
            point_feature =
                Some(create_feature_with_geometry(&keys, &point_geometry));
        }
        if make_line {
            let end_coords = create_coordinates(records[i + 1]);

            let line_coords = Array::new();
            line_coords.push(&coords);
            line_coords.push(&end_coords);

            let line_geometry =
                create_geometry(&keys, &keys.line, &line_coords);
            line_feature =
                Some(create_feature_with_geometry(&keys, &line_geometry));
        }

        if colored_datastream.is_some() {
            let val = colored_datastream.get_stream(records[i]);
            let color = cmaps::get_data_color(val, &cmap_params);
            let props = Object::new();
            Reflect::set(&props, &keys.color, &color.into()).unwrap();

            if make_point {
                Reflect::set(
                    point_feature.as_ref().unwrap(),
                    &keys.properties,
                    &props,
                )
                .unwrap();
            }
            if make_line {
                Reflect::set(
                    line_feature.as_ref().unwrap(),
                    &keys.properties,
                    &props,
                )
                .unwrap();
            }
        }
        if make_point {
            point_features.push(&point_feature.unwrap());
        }
        if make_line {
            line_features.push(&line_feature.unwrap());
        }
    }

    (
        create_geojson(&point_features),
        create_geojson(&line_features),
    )
}

struct ObjKeys {
    obj_type: JsValue,
    feature: JsValue,
    point: JsValue,
    line: JsValue,
    coordinates: JsValue,
    geometry: JsValue,
    color: JsValue,
    properties: JsValue,
}

fn create_coordinates(rec: &common::Location) -> Array {
    let coords = Array::new();
    coords.push(&rec.lon.into());
    coords.push(&rec.lat.into());
    coords
}

/// Create a js_sys::Object with the specified type and coordinates
fn create_geometry(
    keys: &ObjKeys,
    geom_type: &JsValue,
    coords: &Array,
) -> Object {
    let geometry = Object::new();
    Reflect::set(&geometry, &keys.obj_type, geom_type).unwrap();
    Reflect::set(&geometry, &keys.coordinates, coords).unwrap();
    geometry
}

/// Create a js_sys::Object with the specified type and coordinates
fn create_feature_with_geometry(keys: &ObjKeys, geometry: &Object) -> Object {
    let feature = Object::new();
    Reflect::set(&feature, &keys.obj_type, &keys.feature).unwrap();
    Reflect::set(&feature, &keys.geometry, geometry).unwrap();
    feature
}

/// Create the top-level geojson object containing the array of features.
///
/// This is done once at the end of processing records, so memo-izing the string
/// JsValues is not important.
fn create_geojson(features: &Array) -> Object {
    let geojson = Object::new();
    Reflect::set(&geojson, &"type".into(), &"FeatureCollection".into())
        .unwrap();
    Reflect::set(&geojson, &"features".into(), features).unwrap();
    geojson
}

/// Automatically determine the center of the data and a zoom level that will
/// fit it, then fly to that view.
pub fn fly_to_data(map: Rc<Map>, records: &[&common::Location]) {
    // only recenter the map if there's data to zoom to
    if !records.is_empty() {
        let lats: Vec<_> = records.iter().map(|x| x.lat).collect();
        let lons: Vec<_> = records.iter().map(|x| x.lon).collect();
        let (lat_center, lon_center, zoom) = get_view_params(&lats, &lons);
        map.fly_to(&val_to_jsval(&json!({
            "center": [lon_center, lat_center],
            "zoom": zoom,
        })));
    }
}

pub fn fly_to(map: Rc<Map>, lnglat: (f64, f64), zoom: f64) {
    map.fly_to(&val_to_jsval(&json!({
        "center": [lnglat.0, lnglat.1],
        "zoom": zoom,
    })));
}

/// Calculate the center of a map. Does not take the map size into account, so
/// is overly conservative (zooms further out than needed)
fn get_view_params(lats: &[f64], lons: &[f64]) -> (f64, f64, f64) {
    if lats.is_empty() {
        return (0., 0., 0.);
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
    let mut zoom = zoom - 1.;
    zoom = zoom.clamp(0., 16.);
    (lat_center, lon_center, zoom)
}
