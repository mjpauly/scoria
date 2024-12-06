//! Import pins from a geojson file, e.g. one exported from Google Maps.
//!
//! Format must be a FeatureCollection of Point geometries.
//!
//! {
//!   "geometry": {
//!     "coordinates": [
//!       -118.3443238,
//!       34.0706117
//!     ],
//!     "type": "Point"
//!   },
//!   "properties": {
//!     "date": "2024-01-01T00:00:00Z",
//!     "google_maps_url": "http://maps.google.com/?cid=14540715409736475793",
//!     "location": {
//!       "address": "211 S La Brea Ave, Los Angeles, CA 90036, United States",
//!       "country_code": "US",
//!       "name": "Aurora LA"
//!     }
//!   },
//!   "type": "Feature"
//! },

use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use common::{
    pin::Pin,
    popups::{PopUp, PopUpCode, PopUpKind},
    LngLat, ToFront,
};
use geojson::{Feature, FeatureCollection, GeoJson, JsonValue, Value};
use tracing::error;

use crate::{
    app_state::get_front_state,
    database::{
        get_main_db_pool,
        pins::{save_pin_to_db, update_derived_pins},
    },
    ws_session::{send_error_popup, send_message_to_front},
};

/// Read file to string, indicating valid utf8 (true) or lossy utf-8 conversion.
fn read_to_string(path: &PathBuf) -> std::io::Result<(String, bool)> {
    let contents = fs::read(path)?;
    Ok(match String::from_utf8(contents.clone()) {
        Ok(out) => (out, true),
        Err(_) => (String::from_utf8_lossy(&contents).to_string(), false),
    })
}

pub async fn import_pins(path: PathBuf) {
    let (geojson_str, valid_utf8) =
        match read_to_string(&path).context("reading GeoJSON import file") {
            Ok(s) => s,
            Err(e) => {
                error!("{e:?}");
                send_error_popup("Failed to read file.");
                return;
            }
        };
    let geoj = match geojson_str.parse::<GeoJson>() {
        Ok(g) => g,
        Err(e) => {
            error!(
                "Failed to parse file contents as GeoJSON. \
                   (Contained {} UTF-8 data.): {e:?}",
                if valid_utf8 { "valid" } else { "invalid" }
            );
            send_error_popup("Failed to parse file as GeoJSON.");
            return;
        }
    };
    let fc = match FeatureCollection::try_from(geoj) {
        Ok(fc) => fc,
        Err(e) => {
            error!("GeoJSON not a Feature Collection: {e:?}");
            send_error_popup("GeoJSON not a Feature Collection.");
            return;
        }
    };
    let mut pin_default =
        get_front_state(|s| s.pin_import_default.clone()).unwrap_or_default();
    pin_default.id = None; // make sure we're inserting new pins without ids
    let n_features = fc.features.len();
    let mut n_saved = 0;
    for feature in fc.features.into_iter() {
        if let Some(pin) = pin_from_feature(feature, &pin_default) {
            let Ok(conn) = get_main_db_pool() else {
                return;
            };
            if let Err(e) = save_pin_to_db(&conn, &pin).await {
                error!("Failed to save pin. {e}");
            } else {
                n_saved += 1;
            }
        }
    }
    send_result_popup(n_features, n_saved);
    update_derived_pins().await;
}

fn send_result_popup(n_features: usize, n_saved: usize) {
    let code = PopUpCode::Other;
    let popup = if n_saved == n_features {
        let s = if n_saved == 1 { "" } else { "s" };
        PopUp {
            kind: PopUpKind::Success,
            msg: format!("Saved {n_saved} place{s}"),
            code,
        }
    } else if n_saved > 0 {
        PopUp {
            kind: PopUpKind::Error,
            msg: format!(
                "Failed to save some places. Saved \
                {n_saved} of {n_features}.",
            ),
            code,
        }
    } else {
        PopUp {
            kind: PopUpKind::Error,
            msg: "Failed to save places.".into(),
            code,
        }
    };
    send_message_to_front(ToFront::PopUp(popup));
}

/// Extract a Pin from a feature, returning Some(Pin) if successful or None if
/// not.
///
/// The user provides the default pin values to use. to. If the point has
/// coordinates [0,0], then it is given a the default location and a special
/// icon.
fn pin_from_feature(f: Feature, pin_template: &Pin) -> Option<Pin> {
    let geometry = f.geometry.as_ref()?;
    let Value::Point(point) = &geometry.value else {
        return None;
    };
    let lng = *point.first()?;
    let lat = *point.get(1)?;

    let mut pin = pin_template.clone();
    if lng == 0.0 && lat == 0.0 {
        // location unknown, leave at default spot and change icon to make it
        // easier to find
        let _ = pin.set_icon("❓".into());
    } else {
        // valid lnglat
        let _ = pin.set_lnglat(LngLat { lng, lat });
    }

    if let Some(props) = f.properties {
        let mut tags = vec![];
        let mut name = None;
        traverse_props(&JsonValue::Object(props), "", &mut tags, &mut name);
        pin.set_tags(tags);
        if let Some(name) = name {
            pin.set_name(name);
        }
    }

    Some(pin)
}

/// Collects the tags, and populates the name if found in a field called "name".
fn traverse_props(
    obj: &JsonValue,
    path: &str,
    tags: &mut Vec<(String, String)>,
    name: &mut Option<String>,
) {
    match obj {
        JsonValue::Object(obj) => {
            for (key, value) in obj {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                match (key.to_lowercase().as_str(), value) {
                    ("name", JsonValue::String(s)) => *name = Some(s.clone()),
                    _ => traverse_props(value, &new_path, tags, name),
                }
            }
        }
        JsonValue::Array(arr) => {
            for (i, value) in arr.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                traverse_props(value, &new_path, tags, name);
            }
        }
        JsonValue::String(s) => tags.push((path.to_string(), s.clone())),
        _ => tags.push((path.to_string(), obj.to_string())),
    }
}
