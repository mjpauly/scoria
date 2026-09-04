//! Colorbar component for location data

use plotly::common::{ColorScale, ColorScaleElement};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::ui_state::{BackState, FrontState};
use common::{cmaps::Cminmax, map_style::ColoredDataStream};

/// Convert the contrast-trimmed subrange (lo, hi) of a colormap to a plotly
/// colorscale, renormalized so the trimmed colors span the full bar.
pub fn cmap_to_plotly(
    cmap: &[(f64, &'static str)],
    (lo, hi): (f64, f64),
) -> ColorScale {
    let mut scale: Vec<_> = cmap
        .iter()
        .filter(|x| x.0 >= lo && x.0 <= hi)
        .map(|x| ColorScaleElement((x.0 - lo) / (hi - lo), x.1.to_string()))
        .collect();
    // Plotly requires the colorscale to go from 0 to 1 or it shows a default
    // colormap
    scale[0].0 = 0.;
    ColorScale::Vector(scale)
}

#[function_component]
pub fn Colorbar() -> Html {
    let colorbar_id = "colorbar";
    let cmap_params = use_selector(|s: &BackState| s.cmap_params);
    let colored_datastream =
        use_selector(|s: &FrontState| s.map.style.colored_datastream);
    let unit_pref = use_selector(|s: &FrontState| s.unit_pref);

    use_effect_with_deps(
        move |(cmap_params, colored_datastream)| {
            let Cminmax { mut cmin, mut cmax } =
                cmap_params.cminmax.unwrap_or_default();
            // Convert cmax/cmin to preferred units
            match **colored_datastream {
                ColoredDataStream::HorizAccuracy
                | ColoredDataStream::Altitude
                | ColoredDataStream::VertAccuracy => {
                    cmin = unit_pref.small_length.from_base_unit(cmin);
                    cmax = unit_pref.small_length.from_base_unit(cmax);
                }
                ColoredDataStream::Speed | ColoredDataStream::SpeedAccuracy => {
                    cmin = unit_pref.velocity.from_base_unit(cmin);
                    cmax = unit_pref.velocity.from_base_unit(cmax);
                }
                _ => (),
            };

            // Need to handle the case where all data has the same value
            // In this case the binary_search_by using partial_cmp will
            // default to picking the middle color in the colormap, so we just
            // need to make it symmetric around the cmin/cmax value.
            if cmin == cmax {
                cmin -= 1.;
                cmax += 1.;
            }

            // calculate font sizes for responsive sizing
            let system_default_size = 16.;
            let title_default_size = 14;
            let tick_default_size = 12;
            let font_size =
                get_system_font_size().unwrap_or(system_default_size);
            let title_font_size = (font_size * title_default_size as f64
                / system_default_size) as i64;
            let tick_font_size = (font_size * tick_default_size as f64
                / system_default_size) as i64;
            let height_diff = (title_font_size - title_default_size)
                + (tick_font_size - tick_default_size);

            let txt_color = plotly::color::Rgba::new(192, 192, 192, 1.);
            let mut colorbar = plotly::common::ColorBar::new()
                .orientation(plotly::common::Orientation::Horizontal)
                .thickness(15)
                .ticks(plotly::common::Ticks::Outside)
                .tick_angle(0.)
                .tick_color(txt_color)
                .tick_font(
                    plotly::common::Font::new()
                        .color(txt_color)
                        .size(tick_font_size as usize),
                )
                .x_pad(50.)
                .y(0.)
                .title(
                    plotly::common::Title::new(
                        &colored_datastream.name_with_unit(&unit_pref),
                    )
                    .side(plotly::common::Side::Top)
                    .font(
                        plotly::common::Font::new()
                            .color(txt_color)
                            .size(title_font_size as usize),
                    ),
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
                .color_scale(cmap_to_plotly(
                    cmap_params.cmap.cmap_array(),
                    cmap_params.subrange(),
                ))
                .cmin(cmin)
                .cmax(cmax)
                .color_bar(colorbar)
                .opacity(0.0); // hide the single data point

            let trace = plotly::Scatter::new(vec![1.], vec![1.]).marker(marker);
            let bg_color = plotly::color::Rgba::new(0, 0, 0, 0.);
            let layout = plotly::Layout::new()
                .height((80 + height_diff) as usize)
                .font(plotly::common::Font::new().family(r#"ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji""#))
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
            // Purge on cleanup: responsive:true adds a window resize
            // listener that otherwise retains the div (and through the
            // detached DOM tree, the whole unmounted page -- the map leak
            // in doc/decimation/probe-results/churn.md). The element is
            // captured now since it may be detached by cleanup time.
            let gd = crate::plotly::graph_div(colorbar_id);
            move || {
                if let Some(gd) = gd {
                    crate::plotly::purge_element(&gd)
                }
            }
        },
        (cmap_params, colored_datastream),
    );
    html! {
        <div id={colorbar_id}> </div>
    }
}

fn get_system_font_size() -> Option<f64> {
    let window = web_sys::window()?;
    let body_el = window.document()?.body()?;
    let font_size_str = window
        .get_computed_style(&body_el)
        .ok()??
        .get_property_value("font-size")
        .ok()?;
    // remove trailing "px"
    let trimmed = font_size_str.trim_matches(char::is_alphabetic);
    let font_size = trimmed.parse::<f64>().ok()?;
    Some(font_size)
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
