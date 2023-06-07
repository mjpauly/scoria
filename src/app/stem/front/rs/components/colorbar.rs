//! Colorbar component for location data

use plotly::common::{ColorScale, ColorScaleElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::ui_state::{BackState, FrontState};
use common::map_style::ColoredDataStream;

pub fn cmap_to_plotly(cmap: &[(f64, &'static str)]) -> ColorScale {
    let mut scale: Vec<_> = cmap
        .iter()
        .map(|x| ColorScaleElement(x.0, x.1.to_string()))
        .collect();
    // Plotly requires the colorscale to go from 0 to 1 or it shows a default
    // colormap
    scale[0].0 = 0.;
    ColorScale::Vector(scale)
}

#[function_component]
pub fn Colorbar() -> Html {
    let colorbar_id = "colorbar";
    let cmap_params = use_selector(|s: &BackState| s.cmap_params.clone());
    let colored_datastream =
        use_selector(|s: &FrontState| s.map.style.colored_datastream.clone());

    use_effect_with_deps(
        move |(cmap_params, colored_datastream)| {
            let mut cmap_params = (**cmap_params).clone();
            // Need to handle the case where all data has the same value
            // In this case the binary_search_by using partial_cmp will
            // default to picking the middle color in the colormap, so we just
            // need to make it symmetric around the cmin/cmax value.
            if cmap_params.cmin == cmap_params.cmax {
                cmap_params.cmin -= 1.;
                cmap_params.cmax += 1.;
            }

            let mut colorbar = plotly::common::ColorBar::new()
                .orientation(plotly::common::Orientation::Horizontal)
                .thickness(15)
                .ticks(plotly::common::Ticks::Outside)
                .tick_angle(0.)
                .x_pad(50.)
                .y(0.)
                .title(
                    plotly::common::Title::new(
                        &colored_datastream.name_with_unit(),
                    )
                    .side(plotly::common::Side::Top),
                );
            if **colored_datastream == ColoredDataStream::Course {
                colorbar = colorbar
                    .tick_vals(vec![0., 90., 180., 270., 360.])
                    .tick_text(vec!["N", "E", "S", "W", "N"]);
            } else if **colored_datastream == ColoredDataStream::TimeOfDay {
                let vals = [0., 6., 12., 18., 24.]
                    .iter()
                    .map(|x| x * 60. * 60.)
                    .collect::<Vec<f64>>();
                colorbar = colorbar
                    .tick_vals(vals)
                    .tick_text(vec!["0:00", "6:00", "12:00", "18:00", "0:00"])
            }
            // see @maybe_useful_later/time_colorbar.rs for initial code
            // on doing a colorbar for time (plotly doesn't handle it well)
            let marker = plotly::common::Marker::new()
                .color_scale(cmap_to_plotly(cmap_params.cmap.cmap_array()))
                .cmin(cmap_params.cmin)
                .cmax(cmap_params.cmax)
                .color_bar(colorbar)
                .opacity(0.0); // hide the single data point

            let trace = plotly::Scatter::new(vec![1.], vec![1.]).marker(marker);
            let bg_color = plotly::color::Rgba::new(0, 0, 0, 0.);
            let layout = plotly::Layout::new()
                .height(80)
                .paper_background_color(bg_color)
                .plot_background_color(bg_color)
                .margin(
                    plotly::layout::Margin::new()
                        .top(0)
                        .left(0)
                        .bottom(0)
                        .right(0),
                )
                .x_axis(plotly::layout::Axis::new().visible(false))
                .y_axis(plotly::layout::Axis::new().visible(false));
            let config = plotly::configuration::Configuration::new()
                .responsive(true)
                .static_plot(true);
            let mut plot = plotly::Plot::new();
            plot.add_trace(trace);
            plot.set_layout(layout);
            plot.set_configuration(config);

            new_plot_(colorbar_id, &plot.to_js_object()).unwrap();
        },
        (cmap_params, colored_datastream),
    );
    html! {
        <div id={colorbar_id}> </div>
    }
}

use js_sys::Object;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = newPlot)]
    fn new_plot_(id: &str, obj: &Object) -> Result<JsValue, JsValue>;
}

/*
// Plotlyjs colorbar only example
// Codepen link: https://codepen.io/hatbox/pen/rNqKrWL

// Define the colorbar properties
var colorbar = {
  title: {
    text: 'Colorbar Title',
    side: 'top',
  },
  ticks: 'outside',
  //tickvals: [0, 1, 2, 3, 4, 5], // Example tick values
  //ticktext: ['A', 'B', 'C', 'D', 'E', 'F'], // Example tick labels
  thickness: 20,
  y: 0,
  xpad: 50.,
  orientation: 'h',
};

var m = {
  colorbar: colorbar,
  opacity: 0, // hide (also covered by the colorbar)
  cmin: 0,
  cmax: 10.5,
  colorscale: [[0, 'rgb(10,0,50)'], [1, 'rgb(200,200,0)']],
};

var trace = {
  type: 'scatter',
  x: [1],
  y: [1],
  marker: m
};

// Set layout options
var layout = {
  height: 80,
  margin: { t: 0, r: 0, b: 0, l: 0 },
  xaxis: { visible: false },
  yaxis: { visible: false },
  plot_bgcolor: 'rgba(0,0,0,0)', // Transparent background
  paper_bgcolor: 'rgba(0,0,0,0)', // Transparent background
};
var config = {
  responsive: true, // resize if necessary
  staticPlot: true, // non-interactive
}

Plotly.newPlot('myDiv', [trace], layout, config);
*/
