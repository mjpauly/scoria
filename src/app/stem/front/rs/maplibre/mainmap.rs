//! Bindings and data processing for the Maplibre charts.
use std::rc::Rc;

use anyhow::Context;
use common::mounted::MountID;
use gloo_net::http::Request;
use js_sys::{Array, Reflect};
use serde_json::{json, Value};
use tracing::error;
use wasm_bindgen::{prelude::*, JsCast};

use crate::maplibre::{binds::*, pins, selected_points};
use crate::router::get_scoped_host;
use crate::unwrapping::{unwrap_js_result_or_log, unwrap_option_or_log};
use common::cmaps;
use common::map_style::{ColoredDataStream, MapStyle, Rgba};
use common::view_position::ViewPosition;

static UNEXPLORED_SOURCE_ID: &str = "unexplored";
static UNEXPLORED_LAYER_ID: &str = "unexplored";
static LAST_LOCATION_SOURCE_ID: &str = "last_location";
static LAST_LOCATION_LAYER_ID: &str = "last_location";
pub static PINS_SOURCE_ID: &str = "pins";
pub static PINS_LAYER_ID: &str = "pins";

pub fn view_pos_from_map(map: &Map) -> ViewPosition {
    let lnglat_convert = |x: &LngLat| common::LngLat {
        lng: x.lng(),
        lat: x.lat(),
    };
    let center = map.get_center();
    let bounds = map.get_bounds();
    let bounds = common::view_position::LngLatBounds {
        sw: lnglat_convert(&bounds.get_south_west()),
        ne: lnglat_convert(&bounds.get_north_east()),
    };
    ViewPosition {
        center: lnglat_convert(&center),
        zoom: map.get_zoom(),
        bearing: map.get_bearing(),
        pitch: map.get_pitch(),
        bounds,
    }
}

pub fn new_map(
    plot_id: &str,
    style: &Value,
    view_position: &ViewPosition,
    on_load_callback: Box<dyn Fn()>, // closure to run when the plot loads
    // closure to run with pan/zoom data
    on_view_change_callback: Box<dyn Fn(ViewPosition)>,
) -> Rc<Map> {
    // Create the map and start it loading
    let opts = json!({
        "container": plot_id,
        "style": style,
        "center": [view_position.center.lng, view_position.center.lat],
        "zoom": view_position.zoom,
        "bearing": view_position.bearing,
        "pitch": view_position.pitch,
        "doubleClickZoom": false,
        "preserveDrawingBuffer": true, // required to save canvas as png
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

    // Notify the yew component of the new view position whenever it changes.
    let on_view_change: Box<dyn Fn()> = {
        let map = map.clone();
        Box::new(move || on_view_change_callback(view_pos_from_map(&map)))
    };
    map.on("move", &Closure::wrap(on_view_change).into_js_value());

    map
}

// Create the point click callback to register on the map, which returns the id
// of the mounted database layer clicked, the coordinates of the click, and the
// color of the point.
pub fn get_click_point_callback(
    mount_id: MountID,
    click_point: impl Fn((MountID, common::LngLat, Option<String>))
        + Clone
        + 'static,
) -> JsValue {
    let cb = move |event: &JsValue| {
        let features =
            unwrap_js_result_or_log!(Reflect::get(event, &"features".into()));
        let first = unwrap_option_or_log!(features.dyn_ref::<Array>()).at(0);

        // get the color of the data point
        let props = unwrap_js_result_or_log!(Reflect::get(
            &first,
            &"properties".into()
        ));
        let color_obj =
            unwrap_js_result_or_log!(Reflect::get(&props, &"color".into()));
        let color = if !color_obj.is_undefined() {
            Some(unwrap_option_or_log!(color_obj.as_string()))
        } else {
            None
        };

        // get the lng, lat of the data point so we can find it
        let geometry =
            unwrap_js_result_or_log!(Reflect::get(&first, &"geometry".into()));
        let coordinates = unwrap_js_result_or_log!(Reflect::get(
            &geometry,
            &"coordinates".into()
        ));
        let coord_arr = unwrap_option_or_log!(coordinates.dyn_ref::<Array>());
        let lng = unwrap_option_or_log!(coord_arr.at(0).as_f64());
        let lat = unwrap_option_or_log!(coord_arr.at(1).as_f64());

        click_point((mount_id, common::LngLat { lng, lat }, color));
    };
    Closure::wrap(Box::new(cb) as Box<dyn Fn(&JsValue)>).into_js_value()
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

/// Update the map's points and lines data sources. Forces the map to update.
pub fn update_data(map: Rc<Map>, mounted_dbs_enabled: &[MountID]) {
    for (source_id, source_url) in points_lines_ids_and_urls(
        mounted_dbs_enabled,
        &[LayerKind::Points, LayerKind::Lines],
    ) {
        map.get_source(&source_id).set_data(&source_url.into());
    }
}

/// Build the combinations of source/layer ids and source urls for the points and
/// lines for each enabled database.
///
/// Source and layer ids are identical.
///
/// e.g. DB 0 has source ids "points0" and "lines0", and source urls
/// "./0/points.geojson" and "/0/lines.geojson"
pub fn points_lines_ids_and_urls(
    mounted_dbs_enabled: &[MountID],
    layer_kinds: &[LayerKind],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for id in mounted_dbs_enabled {
        for kind in layer_kinds {
            out.push((layer_id(id, kind), source_url(id, kind)));
        }
    }
    out
}

/// Layer and source ids are the same in our app (they can be distinct).
pub fn layer_id(mount_id: &MountID, layer_kind: &LayerKind) -> String {
    format!("{layer_kind}{mount_id}")
}

pub fn source_url(mount_id: &MountID, layer_kind: &LayerKind) -> String {
    format!("./{mount_id}/{layer_kind}.geojson")
}

pub enum LayerKind {
    Points,
    Lines,
}

impl std::fmt::Display for LayerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Points => write!(f, "points"),
            Self::Lines => write!(f, "lines"),
        }
    }
}

/// Restyles the whole plot. Necessary if changing the basemap layer since
/// sources and layers are removed.
pub fn restyle(map: Rc<Map>, style: &Value) {
    map.set_style(&val_to_jsval(style));
}

/// Modify a serde_json::Value object containing the basemap style to add on
/// the user data sources and layers.
pub fn add_source_and_layers_to_style(
    style: &mut Value,
    map_style: &MapStyle,
    mounted_dbs_enabled: &[MountID],
) {
    // Sources
    let sources_mut = style["sources"].as_object_mut().unwrap();
    sources_mut.insert(
        PINS_SOURCE_ID.to_string(),
        geojson_source_with_value(&empty_geojson()),
    );
    sources_mut.insert(
        selected_points::SOURCE_ID.to_string(),
        geojson_source_with_value(&empty_geojson()),
    );
    // add all points and lines sources for each enabled database
    for (source_id, source_url) in points_lines_ids_and_urls(
        mounted_dbs_enabled,
        &[LayerKind::Points, LayerKind::Lines],
    ) {
        sources_mut.insert(source_id, geojson_source_with_url(&source_url));
    }
    if map_style.show_last_location {
        // The last location will be added when the map initializes
        sources_mut.insert(
            LAST_LOCATION_SOURCE_ID.to_string(),
            geojson_source_with_value(&empty_geojson()),
        );
    }
    if map_style.automap {
        sources_mut.insert(UNEXPLORED_SOURCE_ID.to_string(), screen_source());
    }

    // Layers
    // Earlier layers are lower in the map view.
    // Unexplored area first, then lines, then points which go on top
    if map_style.automap {
        let screen = make_screen_layer(style, map_style);
        style["layers"].as_array_mut().unwrap().push(screen);
    }
    let layers_mut = style["layers"].as_array_mut().unwrap();
    if map_style.pins_below_data {
        layers_mut.push(pins::make_pins_layer());
    }
    // add all lines layers for each enabled database
    for (layer_id, _) in
        points_lines_ids_and_urls(mounted_dbs_enabled, &[LayerKind::Lines])
    {
        layers_mut.push(make_lines_layer(
            &layer_id,
            map_style.line_size,
            &map_style.solid_color,
            &map_style.colored_datastream,
        ));
    }
    layers_mut.push(selected_points::make_layer(map_style));
    // add all points layers for each enabled database
    for (layer_id, _) in
        points_lines_ids_and_urls(mounted_dbs_enabled, &[LayerKind::Points])
    {
        layers_mut.push(make_points_layer(
            &layer_id,
            map_style.marker_size,
            &map_style.solid_color,
            &map_style.colored_datastream,
        ));
    }
    if !map_style.pins_below_data {
        layers_mut.push(pins::make_pins_layer());
    }
    if map_style.show_last_location {
        layers_mut.push(make_last_location_layer());
    }
}

fn geojson_source_with_url(url: &str) -> Value {
    json!({
        "type": "geojson",
        "data": url,
    })
}

fn screen_source() -> Value {
    let host = get_scoped_host();
    json!({
        "type": "vector",
        "tiles": [
            format!("http://{host}/screen/tiles/foo/{{z}}/{{x}}/{{y}}.pbf")
        ],
        // max zoom to request tiles from, overzooming if going further in
        "maxzoom": 15,
    })
}

fn make_screen_layer(style: &Value, map_style: &MapStyle) -> Value {
    let outline_color = if map_style.basemap_style.is_dark() {
        "hsl(240, 60%, 80%)" // light, low-saturation blue/purple
    } else {
        "hsl(240, 60%, 40%)" // dark, low-saturation blue/purple
    };
    json!({
        "id": UNEXPLORED_LAYER_ID,
        "type": "fill",
        "source": UNEXPLORED_SOURCE_ID,
        "source-layer": "screen", // layer within the tiles
        // Do not want "minzoom" and "maxzoom" here since they define the range
        // where the tiles are shown
        "paint": {
            "fill-color": get_background_color(style),
            "fill-outline-color": outline_color,
            "fill-opacity": map_style.automap_opacity,
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
pub fn val_to_jsval(v: &Value) -> JsValue {
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
    id: &str,
    marker_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) -> Value {
    json!({
        "id": id,
        "type": "circle",
        "source": id,
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
    id: &str,
    line_size: usize,
    marker_color: &Rgba,
    colored_datastream: &ColoredDataStream,
) -> Value {
    json!({
        "id": id,
        "type": "line",
        "source": id, // source id same as layer id
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

pub fn fly_to(map: Rc<Map>, lnglat: common::LngLat, zoom: f64) {
    map.fly_to(&val_to_jsval(&json!({
        "center": [lnglat.lng, lnglat.lat],
        "zoom": zoom,
    })));
}

/// A geojson object containing a single lnglat point
fn geojson_point(loc: Option<common::LngLat>) -> Value {
    if let Some(loc) = loc {
        json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                     "type": "Point",
                     "coordinates": [ loc.lng, loc.lat ]
                }
            }]
        })
    } else {
        empty_geojson()
    }
}

pub fn empty_geojson() -> Value {
    json!({
        "type": "FeatureCollection",
        "features": []
    })
}

/// The point that shows the last logged location as a blue circle with a white
/// ring.
fn make_last_location_layer() -> Value {
    json!({
        "id": LAST_LOCATION_LAYER_ID,
        "type": "circle",
        "source": LAST_LOCATION_SOURCE_ID,
        "paint": {
            "circle-radius": 5,
            "circle-color": "#0a84ff",
            "circle-opacity": 1.,
            "circle-stroke-width": 3,
            "circle-stroke-color": "#ffffff",
            // "circle-stroke-opacity": 1.,
        }
    })
}

pub fn update_last_location(map: Rc<Map>, loc: Option<common::LngLat>) {
    map.get_source(LAST_LOCATION_SOURCE_ID)
        .set_data(&val_to_jsval(&geojson_point(loc)));
}

fn geojson_source_with_value(value: &Value) -> Value {
    json!({
        "type": "geojson",
        "data": value,
    })
}

/// Generate an image from the current map view and return the data to a
/// callback.
pub fn generate_image(map: Rc<Map>) {
    let callback = Closure::wrap(Box::new(move |blob: web_sys::Blob| {
        yew::platform::spawn_local(async move {
            let resp = match Request::post("./save_image")
                .header("Content-Type", "image/png")
                .body(blob)
                .send()
                .await
                .context("posting save image request")
            {
                Ok(resp) => resp,
                Err(e) => {
                    error!("{e:?}");
                    return;
                }
            };
            if !resp.ok() {
                error!(
                    "save image request failed with {}: {}",
                    resp.status(),
                    resp.status_text()
                )
            }
        });
    }) as Box<dyn Fn(web_sys::Blob)>);

    let canvas = map.get_canvas();
    // use jpeg, which defaults to quality ~0.9
    canvas
        .to_blob_with_type(
            callback.into_js_value().unchecked_ref(),
            "image/jpeg",
        )
        .unwrap();
}
