//! Colorbar component for location data

use yew::prelude::*;

use crate::components::location_filter_list::{apply_filters, Filter};
use crate::components::map_styler::ColoredDataStream;
use crate::plots::cmaps;
use common::Location;

#[derive(Properties, PartialEq)]
pub struct ColorbarProps {
    pub records: UseStateHandle<Vec<Location>>,
    pub filters: UseStateHandle<Vec<Filter>>,
    pub colored_datastream: UseStateHandle<ColoredDataStream>,
}

#[function_component]
pub fn Colorbar(
    ColorbarProps {
        records,
        filters,
        colored_datastream,
    }: &ColorbarProps,
) -> Html {
    let colorbar_id = "colorbar";

    use_effect_with_deps(
        move |(records, filters, colored_datastream)| {
            let recs = apply_filters(filters, records);
            let (cmin, cmax, cmap_to_use) =
                colored_datastream.get_cmap_params(&recs);

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
            }
            // see @maybe_useful_later/time_colorbar.rs for initial code
            // on doing a colorbar for time (plotly doesn't handle it well)
            let marker = plotly::common::Marker::new()
                .color_scale(cmaps::to_plotly(cmap_to_use))
                .cmin(cmin)
                .cmax(cmax)
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
        (records.clone(), filters.clone(), colored_datastream.clone()),
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
