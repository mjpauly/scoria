//! Bindings and data processing for the Maplibre charts.
//!
//! TODO: maplibre just stringifies the json sources anyways. Just do this in
//! backend and link to the geojson source?
use std::rc::Rc;

use js_sys::{Array, Reflect};
use serde_json::{json, Value};
use wasm_bindgen::{prelude::*, JsCast};

use crate::router::get_host;
use common::cmaps;
use common::map_style::{ColoredDataStream, MapStyle, Rgba};
use common::view_position::ViewPosition;

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, PartialEq)]
    pub type Map;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Map)]
    pub fn new(options: &JsValue) -> Map;

    #[wasm_bindgen(method, setter, js_name = showTileBoundaries)]
    pub fn show_tile_boundaries(this: &Map, yes: bool);

    #[wasm_bindgen(method)]
    pub fn on(this: &Map, event: &str, listener: &JsValue);
    #[wasm_bindgen(method, js_name = on)]
    pub fn on_layer(this: &Map, event: &str, layer: &str, listener: &JsValue);
    #[wasm_bindgen(method)]
    pub fn once(this: &Map, event: &str, listener: &JsValue);
    #[wasm_bindgen(method, js_name = once)]
    pub fn once_layer(this: &Map, event: &str, layer: &str, listener: &JsValue);
    #[wasm_bindgen(method)]
    pub fn off(this: &Map, event: &str);
    #[wasm_bindgen(method, js_name = isSourceLoaded)]
    pub fn is_source_loaded(this: &Map, source: &str) -> bool;

    #[wasm_bindgen(method, js_name = getSource)]
    pub fn get_source(this: &Map, id: &str) -> Source;

    #[wasm_bindgen(method, js_name = setStyle)]
    pub fn set_style(this: &Map, style: &JsValue);

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
static POINTS_SOURCE_URL: &str = "./points.geojson";
static LINES_SOURCE_ID: &str = "lines";
static LINES_LAYER_ID: &str = "lines";
static LINES_SOURCE_URL: &str = "./lines.geojson";
static UNEXPLORED_SOURCE_ID: &str = "unexplored";
static UNEXPLORED_LAYER_ID: &str = "unexplored";

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

pub fn new_map(
    plot_id: &str,
    style: &Value,
    view_position: &ViewPosition,
    on_load_callback: Box<dyn Fn()>, // closure to run when the plot loads
    // closure to run with pan/zoom data
    on_view_change_callback: Box<dyn Fn(ViewPosition)>,
    // closure to get popup text and background color given the
    // data point's lng, lat position and color property, if it exists
    request_popup: impl Fn((common::LngLat, Option<String>)) + Clone + 'static,
) -> Rc<Map> {
    // Create the map and start it loading
    let opts = json!({
        "container": plot_id,
        "style": style,
        "center": [view_position.lng, view_position.lat],
        "zoom": view_position.zoom,
        "bearing": view_position.bearing,
        "pitch": view_position.pitch,
        // prevent tile caching so we don't get leak-through of
        // the basemap past the automap screen when zooming out
        // "maxTileCacheSize": 0,
    });
    let map = Map::new(&val_to_jsval(&opts));
    // map.show_tile_boundaries(true); // great for tile debugging

    // Add compass control
    map.add_navigation_control(
        NavigationControl::new(&val_to_jsval(&json!({
            "showCompass": true,
            "showZoom": false,
            "visualizePitch": true,
        }))),
        "bottom-left",
    );

    // Wrap the map in a Rc so we can clone references to it and pass those into
    // closures and share the map reference.
    let map = Rc::new(map);

    // register our on-load callback
    map.on("load", &Closure::wrap(on_load_callback).into_js_value());

    // register our callback for showing a popup on click
    let popup_callback = get_popup_callback(request_popup);
    map.on_layer(
        "click",
        POINTS_LAYER_ID,
        &Closure::wrap(Box::new(popup_callback) as Box<dyn Fn(&JsValue)>)
            .into_js_value(),
    );

    // Notify the yew component of the new view position whenever it changes.
    let on_view_change: Box<dyn Fn()> = {
        let map = map.clone();
        Box::new(move || on_view_change_callback(view_pos_from_map(&map)))
    };
    map.on("moveend", &Closure::wrap(on_view_change).into_js_value());

    map
}

fn get_popup_callback(
    request_popup: impl Fn((common::LngLat, Option<String>)) + Clone + 'static,
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

        request_popup((common::LngLat { lng, lat }, color));
    }
}

pub fn add_popup(
    map: Rc<Map>,
    location: &common::LngLat,
    text: &str,
    bg_color: &str,
) {
    // place popup at exact coordinates of the point
    let newcoords = Array::new();
    newcoords.set(0, location.lng.into());
    newcoords.set(1, location.lat.into());

    let text_color = if cmaps::hex_color_is_bright(bg_color) {
        "#000000" // dark text
    } else {
        "#ffffff"
    };

    // add the css rules for coloring the popup
    style_popup(bg_color, text_color);

    let opts = json!({
        "closeButton": false,
    });
    let popup = Popup::new(&val_to_jsval(&opts));
    popup.set_lng_lat(&newcoords);
    popup.set_html(&text.into());
    popup.add_to(&map);
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

/// Update the map's data source. Forces the map to update, which is useful if
/// the data has changed.
pub fn update_data(map: Rc<Map>) {
    map.get_source(POINTS_SOURCE_ID)
        .set_data(&"./points.geojson".into());
    map.get_source(LINES_SOURCE_ID)
        .set_data(&"./lines.geojson".into());
}

/// Restyles the whole plot. Necessary if changing the basemap layer since
/// sources and layers are removed.
pub fn restyle(map: Rc<Map>, style: &Value) {
    map.set_style(&val_to_jsval(style));
}

/// Modify a serde_json::Value object containing the basemap style to add on
/// the user data sources and layers.
pub fn add_source_and_layers_to_style(style: &mut Value, map_style: &MapStyle) {
    // Sources
    let sources_mut = style["sources"].as_object_mut().unwrap();
    sources_mut.insert(
        POINTS_SOURCE_ID.to_string(),
        geojson_source_with_url(POINTS_SOURCE_URL),
    );
    sources_mut.insert(
        LINES_SOURCE_ID.to_string(),
        geojson_source_with_url(LINES_SOURCE_URL),
    );
    if map_style.automap {
        sources_mut.insert(UNEXPLORED_SOURCE_ID.to_string(), screen_source());
    }

    // Layers
    // Earlier layers are lower in the map view.
    // Unexplored area first, then lines, then points which go on top
    if map_style.automap {
        let screen = make_screen_layer(style);
        style["layers"].as_array_mut().unwrap().push(screen);
    }
    let layers_mut = style["layers"].as_array_mut().unwrap();
    layers_mut.push(make_lines_layer(
        map_style.line_size,
        &map_style.solid_color,
        &map_style.colored_datastream,
    ));
    layers_mut.push(make_points_layer(
        map_style.marker_size,
        &map_style.solid_color,
        &map_style.colored_datastream,
    ));
}

fn geojson_source_with_url(url: &str) -> Value {
    json!({
        "type": "geojson",
        "data": url,
    })
}

fn screen_source() -> Value {
    let host = get_host();
    json!({
        "type": "vector",
        "tiles": [
            format!("{host}/screen/tiles/foo/{{z}}/{{x}}/{{y}}.pbf")
        ],
        // max zoom to request tiles from, overzooming if going further in
        "maxzoom": 15,
    })
}

fn make_screen_layer(style: &Value) -> Value {
    json!({
        "id": UNEXPLORED_LAYER_ID,
        "type": "fill",
        "source": UNEXPLORED_SOURCE_ID,
        "source-layer": "screen", // layer within the tiles
        // Do not want "minzoom" and "maxzoom" here since they define the range
        // where the tiles are shown
        "paint": {
            // "fill-color": "hsl(0, 0%, 17%)",
            "fill-color": get_background_color(style),
            "fill-outline-color": "#ff0", // yellow for visibility
        },
    })
}

/// Retrieve the background color of the style if it exists.
fn get_background_color(style: &Value) -> Value {
    let bg_color = style["layers"][0]["paint"]["background-color"].clone();
    if bg_color.is_null() {
        json!("hsl(0, 0%, 10%)")
    } else {
        bg_color
    }
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

/// Rescales a value in the range 0-1 as if the slider is logarithmically
/// spaced. This is done by taking the linear 0-1 range, exponentating it with
/// the base, then linearly dividing out the base.
///
/// 10.0 is chosen as the base value, which is a close to a typical linear scale
/// but gives more control over small opacity values. A higher value is not
/// desirable, since opacity values below 0.003 are not rendered, and would
/// leave a larger dead zone at the bottom end.
fn log_rescale_opacity(val: f64) -> f64 {
    let base: f64 = 10.0;
    (base.powf(val) - 1.0) / (base - 1.0)
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
            "circle-opacity": log_rescale_opacity(marker_color.a),
            // An invisible stroke of 10px to makes the data points easier to
            // click.
            "circle-stroke-width": 10,
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
            "line-opacity": log_rescale_opacity(marker_color.a),
        },
    })
}

pub fn fly_to(map: Rc<Map>, lnglat: (f64, f64), zoom: f64) {
    map.fly_to(&val_to_jsval(&json!({
        "center": [lnglat.0, lnglat.1],
        "zoom": zoom,
    })));
}
