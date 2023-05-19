use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use serde_json::{json, Value};
use std::time::Duration;
use wasm_bindgen::{prelude::*, JsCast};

use crate::components::map_styler::{ColoredDataStream, Rgba};
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

static SOURCE_ID: &str = "datapoints";
static LAYER_ID: &str = "datapoints";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewPosition {
    pub lng: f64,
    pub lat: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}

impl ViewPosition {
    pub fn from_map(map: &Map) -> Self {
        let center = map.get_center();
        Self {
            lng: center.lng(),
            lat: center.lat(),
            zoom: map.get_zoom(),
            bearing: map.get_bearing(),
            pitch: map.get_pitch(),
        }
    }
}

// TODO: popup on select
#[allow(clippy::too_many_arguments)]
pub fn new_map(
    plot_id: &str,
    basemap: &str,
    marker_size: usize,
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

    // create a geojson without data points
    let geojson = json!({
        "type": "FeatureCollection",
        "features": [],
    });
    let newsource = json!({
        "type": "geojson",
        "data": geojson,
    });
    let newlayer = make_layer(marker_size, marker_color, colored_datastream);

    // Wrap the map in a Rc so we can clone references to it and pass those into
    // closures and share the map reference.
    let map = Rc::new(map);

    let popup_callback = get_popup_callback(map.clone(), get_popup_text);

    // Add the data source and layer and notify the yew component when loading
    // of the map has finished.
    let on_load: Box<dyn Fn()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(SOURCE_ID, &val_to_jsval(&newsource));
            map.add_layer(&val_to_jsval(&newlayer));
            let popup_callback = popup_callback.clone();
            map.on_layer(
                "click",
                LAYER_ID,
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
        Box::new(move || on_view_change_callback(ViewPosition::from_map(&map)))
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
    colored_datastream: &ColoredDataStream,
) {
    let geojson = make_geojson_async(records, colored_datastream).await;
    map.get_source(LAYER_ID).set_data(&geojson);
}

/// Restyle the layer by removing the old layer and adding it back
#[allow(dead_code)]
pub fn restyle_layer(
    map: Rc<Map>,
    marker_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) {
    let newlayer = make_layer(marker_size, marker_color, colored_datastream);
    map.remove_layer(LAYER_ID);
    map.add_layer(&val_to_jsval(&newlayer));
}

/// Restyles the whole plot. Necessary if changing the basemap layer since
/// sources and layers are removed.
#[allow(clippy::too_many_arguments)]
pub async fn restyle(
    map: Rc<Map>,
    records: &[&common::Location],
    basemap: &str,
    marker_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
    callback: Box<dyn Fn()>, // closure to run when restyling is complete
) {
    let geojson = make_geojson_async(records, colored_datastream).await;
    let newsource = Object::new();
    Reflect::set(&newsource, &"type".into(), &"geojson".into()).unwrap();
    Reflect::set(&newsource, &"data".into(), &geojson).unwrap();
    let newlayer = make_layer(marker_size, marker_color, colored_datastream);

    // restyling the basemap also removes our sources and layers, so we need to
    // add them back when the "styledata" event is emitted.
    let on_load: Box<dyn FnMut()> = {
        let map = map.clone();
        Box::new(move || {
            map.add_source(SOURCE_ID, &newsource);
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
    marker_color: &Rgba,
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

/// Sleep for a very short duration to yield execution back to the scheduler,
/// and keep from blocking the UI
pub async fn async_yield() {
    // const for compile-time evaluation
    const SLEEP_DURATION: Duration = Duration::from_micros(1);
    yew::platform::time::sleep(SLEEP_DURATION).await;
}

/// Turn the vector of records into a geojson object
async fn make_geojson_async(
    records: &[&common::Location],
    colored_datastream: &ColoredDataStream,
) -> Object {
    log::debug!("make_geojson_async processing {} records", records.len());
    // yield before computing cmap params
    async_yield().await;
    let cmap_params = colored_datastream.get_cmap_params(records);

    // create the keys to properties once
    let key_type = JsValue::from_str("type");
    let key_feature = JsValue::from_str("Feature");
    let key_point = JsValue::from_str("Point");
    let key_coordinates = JsValue::from_str("coordinates");
    let key_geometry = JsValue::from_str("geometry");
    let key_color = JsValue::from_str("color");
    let key_properties = JsValue::from_str("properties");

    let features = Array::new();

    // Yield at the greater of every 1000 iterations or the total number / 100.
    // We let it increase so as to reduce overhead from the timers at the cost
    // of jank if the number of data points is very large.
    let yield_period = 1000.max(records.len() / 100);
    for (i, rec) in records.iter().enumerate() {
        if i % yield_period == 0 {
            // periodically yield back execution so we don't block too long
            async_yield().await;
        }

        let feature = Object::new();
        Reflect::set(&feature, &key_type, &key_feature).unwrap();

        let geometry = Object::new();
        Reflect::set(&geometry, &key_type, &key_point).unwrap();
        let coords = Array::new();
        coords.push(&rec.lon.into());
        coords.push(&rec.lat.into());
        Reflect::set(&geometry, &key_coordinates, &coords).unwrap();

        Reflect::set(&feature, &key_geometry, &geometry).unwrap();

        if colored_datastream.is_some() {
            let val = colored_datastream.get_stream(rec);
            let color = cmaps::get_data_color(val, &cmap_params);
            let props = Object::new();
            Reflect::set(&props, &key_color, &color.into()).unwrap();
            Reflect::set(&feature, &key_properties, &props).unwrap();
        }
        features.push(&feature);
    }

    let top = Object::new();
    Reflect::set(&top, &"type".into(), &"FeatureCollection".into()).unwrap();
    Reflect::set(&top, &"features".into(), &features).unwrap();
    top
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
