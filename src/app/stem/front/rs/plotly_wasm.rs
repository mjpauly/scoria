//! Additional wasm bindings for plotly beyond newPlot and react, which are
//! already covered in the plotly crate.

use js_sys::Object;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = addTraces)]
    fn add_traces_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = extendTraces)]
    fn extend_traces_(
        id: &str,
        obj: &Object,
        indices: &Object,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(obj: &Object);
}

fn json_to_obj(json: &str) -> Object {
    js_sys::JSON::parse(&json)
        .expect("Invalid JSON")
        .dyn_into::<js_sys::Object>()
        .expect("Invalid JSON structure - expected a top-level Object")
}

/// Add new traces to a plot. `id` is the id of the div containing the plot.
#[allow(dead_code)]
pub fn add_traces(id: &str, trace: Box<dyn plotly::plot::Trace>) {
    let trace_obj = json_to_obj(&trace.to_json());
    add_traces_(id, &trace_obj).expect("Error adding trace");
}

/// Extend a ScatterMapbox with new data
pub fn extend_traces_scattermapbox(id: &str, lat: Vec<f64>, lon: Vec<f64>) {
    // We need to update with a format like this:
    // {"lat":[[37.42884]],"lon":[[-122.17045]]}
    let trace_obj = json_to_obj(
        &(r#"{"lat":["#.to_string()
            + &serde_json::to_string(&lat).unwrap()
            + r#"],"lon":["#
            + &serde_json::to_string(&lon).unwrap()
            + r#"]}"#),
    );
    let indices = json_to_obj("[0]");
    extend_traces_(id, &trace_obj, &indices).expect("Error extending trace");
}
