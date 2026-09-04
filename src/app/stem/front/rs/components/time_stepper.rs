//! Controls for stepping the time range through time: a step-size select,
//! a start/end/both selector, and a scrub ribbon that steps the range once
//! per tap or once per cell-width of drag.

use std::rc::Rc;

use common::time_range::{StepTarget, TimeStep};
use wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};
use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::time_range_picker::now;
use crate::components::{Select, PRIMARY_BUTTON_STYLE, SECONDARY_BUTTON_STYLE};
use crate::ui_state::{BackState, FrontState};
use crate::web::haptics;

/// Pixels of horizontal drag per step.
const DRAG_STEP_PX: f64 = 24.0;
/// Maximum movement for a pointer gesture to count as a tap.
const TAP_SLOP_PX: f64 = 8.0;

static SCRUB_BACK_ID: &str = "time_scrub_back";
static SCRUB_FWD_ID: &str = "time_scrub_fwd";

#[function_component]
pub fn TimeStepper() -> Html {
    html! {
        <div class="my-1">
            <StepConfigRow />
            <ScrubRibbon />
        </div>
    }
}

/// The step-size select and target-end buttons, on one line.
#[function_component]
fn StepConfigRow() -> Html {
    let step = use_selector(|s: &FrontState| s.map.time_step);
    let selected = use_selector(|s: &FrontState| s.map.time_step_target);
    let dispatch = Dispatch::<FrontState>::new();
    let step_onchange = dispatch.reduce_mut_callback_with(
        |s: &mut FrontState, choice: TimeStep| s.map.time_step = choice,
    );
    let buttons = [
        (StepTarget::Start, "Start"),
        (StepTarget::Both, "Both"),
        (StepTarget::End, "End"),
    ]
    .iter()
    .map(|(target, label)| {
        let target = *target;
        let style = if target == *selected {
            PRIMARY_BUTTON_STYLE
        } else {
            SECONDARY_BUTTON_STYLE
        };
        let onclick =
            dispatch.reduce_mut_callback(move |s: &mut FrontState| {
                s.map.time_step_target = target;
            });
        html! {
            <button class={format!("m-0.5 py-1 px-3 {style}")}
                id={format!("time_step_target_{}", label.to_lowercase())}
                onclick={onclick}
            >
                {*label}
            </button>
        }
    })
    .collect::<Html>();
    html! {
        <div class="flex justify-center items-center">
            <Select<TimeStep>
                selection={*step}
                choices={TimeStep::ALL.to_vec()}
                onchange={step_onchange}
                class="m-0.5 mr-2"
                id="time_step_select"
            />
            {buttons}
        </div>
    }
}

/// Bookkeeping for an in-progress scrub gesture.
struct Drag {
    origin_x: f64,
    /// steps already applied during this drag
    applied: i64,
    /// id of the element the gesture started on, for tap handling
    target_id: String,
}

#[function_component]
fn ScrubRibbon() -> Html {
    let step = use_selector(|s: &FrontState| s.map.time_step);
    let map_tz = use_selector(|s: &BackState| s.map_tz.clone());
    // RefCell rather than use_state: pointermove events can outpace
    // re-renders, and each one needs the up-to-date applied count
    let drag = use_mut_ref(|| Option::<Drag>::None);
    // accumulated steps of the active drag, for display only
    let shown_steps = use_state(|| 0i64);
    // raw horizontal drag distance, for shifting the tick ruler
    let drag_px = use_state(|| 0.0f64);

    let apply = {
        let map_tz = map_tz.clone();
        // one tick per application, not per step: a fast swipe should feel
        // like discrete detents, not a buzz
        Rc::new(move |n: i64| {
            haptics::tick();
            let map_tz = map_tz.clone();
            Dispatch::<FrontState>::new().reduce_mut(
                move |s: &mut FrontState| step_time_range(s, &map_tz, n),
            );
        })
    };

    let ribbon_ref = use_node_ref();
    let onpointerdown = {
        let drag = drag.clone();
        let ribbon_ref = ribbon_ref.clone();
        Callback::from(move |e: PointerEvent| {
            // Capture the pointer on the ribbon itself so the drag keeps
            // tracking outside it. It must not be captured through
            // e.current_target(): under yew's delegated events that is
            // yew's root element, and capturing there retargets the
            // pointermove/pointerup events away from these handlers, so
            // the drag would neither track nor end.
            if let Some(elem) = ribbon_ref.cast::<Element>() {
                let _ = elem.set_pointer_capture(e.pointer_id());
            }
            haptics::prepare();
            let target_id = e
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .map(|elem| elem.id())
                .unwrap_or_default();
            *drag.borrow_mut() = Some(Drag {
                origin_x: e.client_x() as f64,
                applied: 0,
                target_id,
            });
        })
    };

    let onpointermove = {
        let drag = drag.clone();
        let shown_steps = shown_steps.clone();
        let drag_px = drag_px.clone();
        let apply = apply.clone();
        Callback::from(move |e: PointerEvent| {
            let mut slot = drag.borrow_mut();
            if slot.is_none() {
                return;
            }
            // a move with no button held means the pointerup was missed;
            // drop the stale drag instead of stepping
            if e.buttons() == 0 {
                *slot = None;
                shown_steps.set(0);
                drag_px.set(0.0);
                return;
            }
            let d = slot.as_mut().unwrap();
            let dx = e.client_x() as f64 - d.origin_x;
            drag_px.set(dx);
            // carousel semantics: the ribbon's contents follow the finger,
            // so dragging right pulls earlier cells toward the center
            let steps = (-dx / DRAG_STEP_PX).round() as i64;
            if steps != d.applied {
                apply(steps - d.applied);
                d.applied = steps;
                shown_steps.set(steps);
            }
        })
    };

    let end_drag = {
        let drag = drag.clone();
        let shown_steps = shown_steps.clone();
        let drag_px = drag_px.clone();
        Callback::from(move |e: PointerEvent| {
            let Some(d) = drag.borrow_mut().take() else {
                return;
            };
            // an unmoved press-and-release on a side cell is a tap: one step
            let dx = e.client_x() as f64 - d.origin_x;
            if d.applied == 0 && dx.abs() < TAP_SLOP_PX {
                if d.target_id == SCRUB_BACK_ID {
                    apply(-1);
                } else if d.target_id == SCRUB_FWD_ID {
                    apply(1);
                }
            }
            shown_steps.set(0);
            drag_px.set(0.0);
        })
    };

    // While dragging, the cells show the accumulated offset around the
    // current step count; idle, they show -1 | 0 | +1 steps.
    let label = {
        let step = *step;
        move |n: i64| {
            let amount = step.value() * n;
            if amount == 0 {
                format!("0{}", step.unit())
            } else {
                format!("{:+}{}", amount, step.unit())
            }
        }
    };
    let cell_style = "flex-1 text-center m-0.5 py-1.5";
    // A faint ruler under the cells, one tick per drag step, hinting that
    // the ribbon scrubs. Each tile is one step wide with the tick at its
    // middle; centering the tiled background puts a tick at the ribbon's
    // center and keeps the ruler symmetric at any width. During a drag the
    // ruler translates with the finger; the tile width matches the step
    // size, so it realigns exactly as each step lands.
    let tick_style = format!(
        "background-image: linear-gradient(to right, \
            transparent {lo}px, rgb(115 115 115 / 0.4) {lo}px, \
            rgb(115 115 115 / 0.4) {hi}px, transparent {hi}px); \
        background-size: {DRAG_STEP_PX}px 100%; \
        background-position: calc(50% + {dx}px) 0;",
        lo = DRAG_STEP_PX / 2.0 - 0.5,
        hi = DRAG_STEP_PX / 2.0 + 0.5,
        dx = *drag_px,
    );
    html! {
        <div class="relative flex w-72 max-w-full mx-auto select-none \
                cursor-ew-resize"
            style="touch-action: none;"
            id="time_scrub_ribbon"
            ref={ribbon_ref}
            onpointerdown={onpointerdown}
            onpointermove={onpointermove}
            onpointerup={end_drag.clone()}
            onpointercancel={end_drag}
        >
            <div id={SCRUB_BACK_ID}
                class={format!("{cell_style} {SECONDARY_BUTTON_STYLE}")}
            >
                {label(*shown_steps - 1)}
            </div>
            <div class={format!("{cell_style} text-neutral-400")}>
                {label(*shown_steps)}
            </div>
            <div id={SCRUB_FWD_ID}
                class={format!("{cell_style} {SECONDARY_BUTTON_STYLE}")}
            >
                {label(*shown_steps + 1)}
            </div>
            <div class="absolute inset-x-3 bottom-1 h-1 pointer-events-none"
                style={tick_style}
            />
        </div>
    }
}

/// Step the map's time range by n steps of the configured step size. The
/// step is applied to the concrete time_range, and the persisted delta
/// range is re-derived from the result, as the manual datetime inputs do.
fn step_time_range(s: &mut FrontState, tz: &str, n: i64) {
    let span = s.map.time_step.span(n);
    let target = s.map.time_step_target;
    let stepped = match s.map.time_range.stepped(span, target, tz) {
        Ok(Some(range)) => range,
        // the step would put start after end; ignore it
        Ok(None) => return,
        Err(e) => {
            tracing::error!("stepping time range: {e:?}");
            return;
        }
    };
    s.map.time_range = stepped;
    let now = now(tz);
    if target.moves_start() {
        if let Ok(start) = stepped.start.intz(tz) {
            if let Ok(offset) = now.until(&start) {
                s.map.time_delta_range.start_offset = offset;
            }
        }
        s.map.time_delta_range.snap_start_to_day = false;
    }
    if target.moves_end() {
        if let Ok(end) = stepped.end.intz(tz) {
            if let Ok(offset) = now.until(&end) {
                s.map.time_delta_range.end_offset = offset;
            }
        }
        s.map.time_delta_range.snap_end_to_day = false;
    }
}
