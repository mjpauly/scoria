//! Colorbar component for location data, using the HTML Canvas element

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement};
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
    let cv_ref = use_node_ref();

    let left_tick_ref = use_node_ref();

    // for E, S, W
    let midleft_tick_ref = use_node_ref();
    let mid_tick_ref = use_node_ref();
    let midright_tick_ref = use_node_ref();

    let right_tick_ref = use_node_ref();

    // size and spacing
    // let h_title = 20; // space for the name of the colored datastream
    let h_bar = 20; // space for the colorbar

    // let h_ticks = 20; // space for the tick labels
    // let px = 50; // padding on the sides to gives space for tick labels
    let w_bar = 256; // the number of steps to draw the bar with
    {
        let cv_ref = cv_ref.clone();
        let left_tick_ref = left_tick_ref.clone();
        let midleft_tick_ref = midleft_tick_ref.clone();
        let mid_tick_ref = mid_tick_ref.clone();
        let midright_tick_ref = midright_tick_ref.clone();
        let right_tick_ref = right_tick_ref.clone();
        use_effect_with_deps(
            move |(records, filters, colored_datastream)| {
                // === Colorbar === //

                let cv = cv_ref.cast::<HtmlCanvasElement>().unwrap();
                let ctx = cv
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .unwrap();

                let recs = apply_filters(filters, records);
                let (cmin, cmax, cmap_to_use) = colored_datastream.get_cmap_params(&recs);

                for i in 0..w_bar {
                    let val = i as f64 / w_bar as f64;
                    let color = cmaps::find_nearest(cmap_to_use, val);

                    ctx.begin_path();

                    ctx.set_fill_style(&color.into());
                    ctx.fill_rect(i as f64, 0., 1., h_bar as f64)
                }

                // === Ticks === //

                /* (incomplete) absolutely position the ticks under the bar ends
                let bounds = cv.get_bounding_client_rect();
                let l = bounds.left();
                let r = bounds.right();
                log::debug!("l, r: {}, {}", l, r);
                */

                let l_elem = left_tick_ref.cast::<HtmlDivElement>().unwrap();
                let ml_elem = midleft_tick_ref.cast::<HtmlDivElement>().unwrap();
                let m_elem = mid_tick_ref.cast::<HtmlDivElement>().unwrap();
                let mr_elem = midright_tick_ref.cast::<HtmlDivElement>().unwrap();
                let r_elem = right_tick_ref.cast::<HtmlDivElement>().unwrap();

                if **colored_datastream == ColoredDataStream::Course {
                    l_elem.set_inner_text("N");
                    ml_elem.set_inner_text("E");
                    m_elem.set_inner_text("S");
                    mr_elem.set_inner_text("W");
                    r_elem.set_inner_text("N");
                } else {
                    // Set tick labels
                    l_elem.set_inner_text(&colored_datastream.format_value(cmin));
                    r_elem.set_inner_text(&colored_datastream.format_value(cmax));
                    for e in [&ml_elem, &m_elem, &mr_elem] {
                        e.set_inner_text("");
                    }
                }
            },
            (records.clone(), filters.clone(), colored_datastream.clone()),
        )
    };
    html! {
        <div class="flex">
            <div class="mx-auto">
                <div class="h-5 flex flex-col">
                    <div class="text-xs text-neutral-500 my-auto">
                        {colored_datastream.to_string()}
                    </div>
                </div>
                <canvas
                    width={format!("{}", w_bar)}
                    height={format!("{}", h_bar)}
                    ref={cv_ref}>
                        /*
                        <div ref={left_tick_ref}>
                            {"left"}
                        </div>
                        <div ref={right_tick_ref}>
                            {"right"}
                        </div>
                        */
                </canvas>
                // /*
                <div class="h-5 flex flex-col">
                    <div class="text-xs text-neutral-500 \
                            my-auto flex justify-between">
                        <div ref={left_tick_ref}>
                            {"left"}
                        </div>
                        <div ref={midleft_tick_ref}>
                            {"midleft"}
                        </div>
                        <div ref={mid_tick_ref}>
                            {"mid"}
                        </div>
                        <div ref={midright_tick_ref}>
                            {"midright"}
                        </div>
                        <div ref={right_tick_ref}>
                            {"right"}
                        </div>
                    </div>
                </div>
                // */
            </div>
        </div>
    }
}
