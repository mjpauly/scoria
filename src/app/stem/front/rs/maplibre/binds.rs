//! Bindings for Maplibre JS API.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, PartialEq)]
    pub type Map;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_name = Map)]
    pub fn new(options: &JsValue) -> Map;
    #[wasm_bindgen(method)]
    pub fn remove(this: &Map);

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
    // NOTE: off() must use the same set of parameters as the on() call that is
    // used. specifically meaning the on()/off() overloading in JS
    pub fn off(this: &Map, event: &str, layer: &str, listener: &JsValue);
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
    #[wasm_bindgen(method, js_name = getBounds)]
    pub fn get_bounds(this: &Map) -> LngLatBounds;

    #[wasm_bindgen(method, js_name = flyTo)]
    pub fn fly_to(this: &Map, options: &JsValue);
    #[wasm_bindgen(method, js_name = easeTo)]
    pub fn ease_to(this: &Map, options: &JsValue);

    #[wasm_bindgen(method, js_name = addImage)]
    pub fn add_image(this: &Map, id: &str, elem: &JsValue);
    #[wasm_bindgen(method, js_name = hasImage)]
    pub fn has_image(this: &Map, id: &str) -> bool;

    #[wasm_bindgen(method, js_name = getCanvas)]
    pub fn get_canvas(this: &Map) -> web_sys::HtmlCanvasElement;

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

    pub type LngLatBounds;

    #[wasm_bindgen(method, js_name = getSouthWest)]
    pub fn get_south_west(this: &LngLatBounds) -> LngLat;
    #[wasm_bindgen(method, js_name = getNorthEast)]
    pub fn get_north_east(this: &LngLatBounds) -> LngLat;

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
