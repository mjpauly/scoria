//! Wasm bindings for plotly.
//!
//! Certain functions like `restyle` require extra nesting on the data, so new
//! "update" trace structs are defined where needed.

use js_sys::Object;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Bindings to plotly functions.
///
/// See https://plotly.com/javascript/plotlyjs-function-reference/
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = newPlot)]
    fn new_plot_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = restyle)]
    fn restyle_(
        id: &str,
        obj: &Object,
        indices: &Object,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = relayout)]
    fn relayout_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = addTraces)]
    fn add_traces_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = deleteTraces)]
    fn delete_traces_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = extendTraces)]
    fn extend_traces_(
        id: &str,
        obj: &Object,
        indices: &Object,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = Plotly)]
    pub type PlotlyHTMLElement;

    #[wasm_bindgen(method)]
    pub fn on(this: &PlotlyHTMLElement, event: &str, callback: JsValue);
}

fn json_to_obj(json: &str) -> Object {
    js_sys::JSON::parse(json)
        .expect("Invalid JSON")
        .dyn_into::<js_sys::Object>()
        .expect("Invalid JSON structure - expected a top-level Object")
}

/// Produce a new pot in the given div.
pub fn new_plot(id: &str, plot: &plotly::Plot) {
    let plot_obj = &plot.to_js_object();
    // log::debug!("new plot json: {}", plot.to_json());
    new_plot_(id, plot_obj).expect("Error plotting chart");
}

/// Replace old data in a scattermapbox with new data. The trace must be an
/// "update" trace with extra nesting on the data.
#[allow(dead_code)]
pub fn restyle(id: &str, trace: Box<dyn plotly::plot::Trace>) {
    let trace_obj = json_to_obj(&trace.to_json());
    let indices = json_to_obj("[0]");
    restyle_(id, &trace_obj, &indices).expect("Error plotting chart");
}

/// Relayout the plot. Seems to trigger resizes only after the window has
/// already been resized once.
#[allow(dead_code)]
pub fn relayout(id: &str, layout: plotly::Layout) {
    let layout_obj = json_to_obj(&layout.to_json());
    log::debug!("new layout json: {}", layout.to_json());
    relayout_(id, &layout_obj).expect("Error relayouting chart");
}

/// Add a new trace to a plot. `id` is the id of the div containing the plot.
#[allow(dead_code)]
pub fn add_trace(id: &str, trace: Box<dyn plotly::plot::Trace>) {
    let trace_obj = json_to_obj(&trace.to_json());
    add_traces_(id, &trace_obj).expect("Error adding trace");
}

/// Delete the first trace index. `id` is the id of the div containing the plot.
#[allow(dead_code)]
pub fn delete_trace(id: &str) {
    let index = json_to_obj("[0]");
    delete_traces_(id, &index).expect("Error adding trace");
}

/// Extend an existing trace with additional data.
pub fn extend_trace(id: &str, trace: Box<dyn plotly::plot::Trace>) {
    let trace_obj = json_to_obj(&trace.to_json());
    // log::debug!("extend trace json: {}", trace.to_json());
    let indices = json_to_obj("[0]");
    extend_traces_(id, &trace_obj, &indices).expect("Error extending trace");
}

/// Add an event listener to a plot id. Panics if the plot element can't be
/// found.
pub fn add_plot_event_listener(
    id: &str,
    event: &str,
    callback: Box<dyn FnMut(JsValue)>,
) {
    let e = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap();
    let plotly_elem = e.unchecked_into::<PlotlyHTMLElement>();
    let cb = Closure::wrap(callback);
    plotly_elem.on(event, cb.into_js_value());
}

/// Relayout data obtained from a relayout event on a mapbox plot
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapboxRelayoutData {
    #[serde(rename = "mapbox.center")]
    pub center: MapboxCenter,
    #[serde(rename = "mapbox.zoom")]
    pub zoom: f64,
    #[serde(rename = "mapbox.bearing")]
    pub bearing: f64,
    #[serde(rename = "mapbox.pitch")]
    pub pitch: f64,
}

/// Relayout data obtained from a relayout event on a mapbox plot
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapboxCenter {
    pub lat: f64,
    pub lon: f64,
}
