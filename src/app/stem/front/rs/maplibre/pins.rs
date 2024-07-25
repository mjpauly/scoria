//! Maplibre-specific functions for displaying user-saved pins to the map. Icons
//! are individual emojis (or any unicode character), loaded as bitmap images on
//! the map.
//!
//! This is a little more convoluted than would be ideal. Images are only loaded
//! on the map once it has been initialized. So we can't have pin data in the
//! style object, since images would be referenced that haven't been loaded yet,
//! producing an error.
//!
//! Restyling thus wipes out the pin source data, and we have to update the pin
//! data again after that.

use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use common::{pin::Pin, LngLat};
use js_sys::{Array, Reflect};
use serde_json::{json, Value};
use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen::{prelude::*, JsCast};

use crate::maplibre::binds;
use crate::maplibre::{val_to_jsval, PINS_LAYER_ID, PINS_SOURCE_ID};

/// Get the list of pins to actually show on the map.
///
/// The pin being edited "shadows" the data from the backend until the changes
/// are saved. If it's a new pin being edited, it is just added to the list so
/// it can be displayed.
pub fn get_visible_pins(
    pins: Vec<Pin>,
    current_pin: Pin,
    editing: bool,
) -> Vec<Pin> {
    let mut newpins = pins
        .into_iter()
        .filter(|p| p.id != current_pin.id)
        .collect::<Vec<_>>();
    if editing || current_pin.id.is_some() {
        // not editing the unsaved pin -> don't show it
        // (e.g. user canceled after creating a new pin)
        newpins.push(current_pin);
    }
    newpins
}

/// Update the pins on the map by loading any missing images and setting the
/// geojson source data.
pub fn update_pins(map: &Rc<binds::Map>, pins: &[Pin]) -> Result<(), JsValue> {
    for pin in pins.iter() {
        load_emoji_image(map, &pin.icon)?;
    }
    map.get_source(PINS_SOURCE_ID)
        .set_data(&val_to_jsval(&make_pins_geojson(pins)));

    Ok(())
}

pub fn update_pins_after_restyle(map: Rc<binds::Map>, pins: Vec<Pin>) {
    map.clone().once(
        "styledata",
        &Closure::wrap(Box::new(move || {
            update_pins(&map, &pins).unwrap();
        }) as Box<dyn Fn()>)
        .into_js_value(),
    );
}

// Load an emoji image into the map, using the emoji as the image identifier.
fn load_emoji_image(map: &Rc<binds::Map>, emoji: &str) -> Result<(), JsValue> {
    let icon_id = icon_id(emoji);
    if map.has_image(&icon_id) {
        // already has this emoji loaded -> return
        return Ok(());
    }

    // canvas sizes (set `icon-size in layout to set display size)
    let h = 200; // canvas height in px
    let char_size = h * 4 / 5; // character size
    let char_width = if cfg!(feature = "android_config") {
        // on android emojis are significantly wider for some reason
        h
    } else {
        char_size
    };
    let w = 20 + char_width * count_characters(emoji) as u32; // canvas width

    let window = web_sys::window().unwrap();

    // Create a canvas element
    let canvas = window
        .document()
        .unwrap()
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    canvas.set_width(w);
    canvas.set_height(h);
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    // display the bounding rectangle for debugging
    // context.rect(0., 0., w as f64, h as f64);
    // context.set_fill_style(&JsValue::from("red"));
    // context.fill();

    context.set_font(&format!("{}px Arial", char_size));
    // center horizontally
    context.set_text_align("center");
    // ideographic is just about the bottom of the emoji
    // https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/textBaseline
    context.set_text_baseline("ideographic");
    context.fill_text(emoji, (w / 2) as f64, (h as f64) * 0.95)?;

    map.add_image(
        &icon_id,
        &JsValue::from(context.get_image_data(0., 0., w as f64, h as f64)?),
    );

    Ok(())
}

/// Count the number of graphemes (user-perceived characters) in a string.
///
/// Emoji usually have a "variation selector" combining mark that indicates how
/// it should be displayed, or can be joined with other emoji using a zero-width
/// joiner. Counting graphemes gives a better estimate of the width of the
/// string.
///
/// E.g. a `heart on fire emoji` (❤️‍🔥) consists of a `heart` (❤)
/// character, an `image` variation selector that indicates it should be
/// rendered like an emoji with color (❤️), a `zero-width joiner`, and a `fire`
/// (🔥) emoji. It takes 13 bytes of space.
fn count_characters(emoji: &str) -> usize {
    emoji.graphemes(true).count()
}

/// Get the image identifier for an emoji.
///
/// The "_icon_" prefix helps ensure we don't have a conflict with other icons
/// created for other purposes, since the user can enter any icon string they
/// want.
fn icon_id(emoji: &str) -> String {
    format!("_icon_{emoji}")
}

/// Make the pins geojson source with all pin locations.
fn make_pins_geojson(pins: &[Pin]) -> Value {
    let features = pins
        .iter()
        .map(|p| {
            json!({
                "type": "Feature",
                "properties": {
                    "id": &p.id,
                    "name": p.name,
                    "icon": &icon_id(&p.icon),
                },
                "geometry": {
                     "type": "Point",
                     "coordinates": [ p.lnglat.lng, p.lnglat.lat ]
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "FeatureCollection",
        "features": features,
    })
}

/// A layer displaying pins on the map as icons.
///
/// https://maplibre.org/maplibre-style-spec/layers/#icon-size
pub fn make_pins_layer() -> Value {
    json!({
        "id": PINS_LAYER_ID,
        "type": "symbol",
        "source": PINS_SOURCE_ID,
        "layout": {
            "icon-image": ["get", "icon"],
            "icon-size": 0.125, // set the displayed icon size
            "icon-padding": 0, // px padding for colission detection
            "icon-overlap": "always",
            // icon-halo-color, icon-halo-width for select?
            /*
            "text-field": ["get", "name"],
            "text-optional": true,
            "text-size": 12,
            "text-overlap": "never",
            */
        },
        /*
        "paint": {
            "text-color": "rgb(255, 255, 255)",
            "text-halo-color": "rgb(0, 0, 0)",
            "text-halo-width": 1,
            "text-translate": [0, 22],
            "text-translate-anchor": "viewport",
        }
        */
    })
}

pub fn register_callbacks(
    map: &Rc<binds::Map>,
    pin_onclick: impl Fn(Option<i64>) + Clone + 'static,
    on_create_pin: impl Fn(LngLat) + Clone + 'static,
) {
    register_click_callback(map, pin_onclick);
    // and for creating a pin on a long press
    register_create_pin_callback(map, on_create_pin);
}

fn register_click_callback(
    map: &Rc<binds::Map>,
    callback: impl Fn(Option<i64>) + Clone + 'static,
) {
    let pin_callback = move |event: &JsValue| {
        let features = Reflect::get(event, &"features".into()).unwrap();
        let first = features.dyn_ref::<Array>().unwrap().at(0);

        let props = Reflect::get(&first, &"properties".into()).unwrap();
        let id_obj = Reflect::get(&props, &"id".into()).unwrap();
        let id = id_obj.as_f64().map(|x| x as i64);
        callback(id);
    };
    map.on_layer(
        "click",
        PINS_LAYER_ID,
        &Closure::wrap(Box::new(pin_callback) as Box<dyn Fn(&JsValue)>)
            .into_js_value(),
    );
}

// set to true when a touch interaction starts, but gets set to false if
// anything other than a long press happens (like touchmove)
static TIMEOUT: Mutex<bool> = Mutex::new(false);

/// Pin creation is triggered by clicking and holding for 1 second.
fn register_create_pin_callback(
    map: &Rc<binds::Map>,
    callback: impl Fn(LngLat) + Clone + 'static,
) {
    let pin_callback = move |event: JsValue| {
        let callback = callback.clone();
        yew::platform::spawn_local(async move {
            *TIMEOUT.lock().unwrap() = true;
            // check every 100ms for ~1s if the creation has be canceled
            for _ in 0..8 {
                yew::platform::time::sleep(Duration::from_millis(100)).await;
                if !*TIMEOUT.lock().unwrap() {
                    // canceled by another interaction
                    return;
                }
            }

            let loc = Reflect::get(&event, &"lngLat".into()).unwrap();
            let x = loc.unchecked_into::<binds::LngLat>();
            callback(LngLat {
                lng: x.lng(),
                lat: x.lat(),
            });
        });
    };
    let create_pin_closure =
        Closure::wrap(Box::new(pin_callback) as Box<dyn Fn(JsValue)>)
            .into_js_value();
    map.on("touchstart", &create_pin_closure);
    map.on("mousedown", &create_pin_closure);
    let cancel_closure = Closure::wrap(Box::new(|_: &JsValue| {
        *TIMEOUT.lock().unwrap() = false;
    }) as Box<dyn Fn(&JsValue)>)
    .into_js_value();
    map.on("touchend", &cancel_closure);
    map.on("touchcancel", &cancel_closure);
    map.on("touchmove", &cancel_closure);
    map.on("mouseup", &cancel_closure);
    map.on("mousemove", &cancel_closure);
    map.on("move", &cancel_closure);
}
