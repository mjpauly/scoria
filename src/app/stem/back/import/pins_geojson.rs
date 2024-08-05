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

use common::{pin::Pin, LngLat};
use geojson::{Feature, FeatureCollection, GeoJson, JsonValue, Value};
use tracing::error;

use crate::{
    app_state::get_front_state,
    database::{
        get_db_pool,
        pins::{save_pin_to_db, update_derived_pins},
    },
};

// TODO: error popup message?

pub async fn import_pins(path: PathBuf) {
    let geojson_str = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to read GeoJSON import file: {e}");
            return;
        }
    };
    let geoj = match geojson_str.parse::<GeoJson>() {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to parse file contents as GeoJSON: {e}");
            return;
        }
    };
    let fc = match FeatureCollection::try_from(geoj) {
        Ok(fc) => fc,
        Err(e) => {
            error!("GeoJSON not a Feature Collection: {e}");
            return;
        }
    };
    let mut pin_default =
        get_front_state(|s| s.map.current_pin.clone()).unwrap_or_default();
    pin_default.id = None; // make sure we're inserting new pins without ids
    for feature in fc.features.into_iter() {
        if let Some(pin) = pin_from_feature(feature, &pin_default) {
            if let Err(e) = save_pin_to_db(&get_db_pool(), &pin).await {
                error!("Failed to save pin. {e}");
            }
        }
    }
    update_derived_pins().await;
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
