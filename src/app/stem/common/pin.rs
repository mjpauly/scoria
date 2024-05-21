use serde::{Deserialize, Serialize};

use crate::LngLat;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: Option<i64>,
    pub lnglat: LngLat, // location of the icon / name
    pub name: String,
    pub icon: String,                  // emoji icon
    pub lists: Vec<String>,            // json array of strings
    pub tags: Vec<(String, String)>,   // key value pairs
    pub boundary: Option<Vec<LngLat>>, // currently unused
}

/// Default to a star emoji and "Untitled" name.
impl Default for Pin {
    fn default() -> Self {
        Self {
            id: Default::default(),
            lnglat: Default::default(),
            name: "Untitled".into(),
            icon: "⭐️".into(),
            lists: Default::default(),
            tags: Default::default(),
            boundary: Default::default(),
        }
    }
}
