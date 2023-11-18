//! A position from which the map is being viewed.

use crate::LngLat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ViewPosition {
    pub center: LngLat,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
    pub bounds: LngLatBounds,
}

/// A latitude/longitude bounding box, defined by its southwest and northeast
/// corners. In Maplibre, these corners can have longitude values greater than
/// 180 or less than -180 when the antimeridian is visible. When determining if
/// a point is contained inside the bounds we thus have to check wrapped
/// longitude values as well
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LngLatBounds {
    pub sw: LngLat,
    pub ne: LngLat,
}

impl LngLatBounds {
    pub fn contains(&self, x: &LngLat) -> bool {
        let inside = |x, lower, upper| (x >= lower) && (x <= upper);
        inside(x.lat, self.sw.lat, self.ne.lat) // lat inside
            && (inside(x.lng, self.sw.lng, self.ne.lng) // lng wrapped and not
                || inside(x.lng + 360.0, self.sw.lng, self.ne.lng)
                || inside(x.lng - 360.0, self.sw.lng, self.ne.lng))
    }

    /// Expand the width and height of the LngLatBounds by the provided factor.
    /// The expansion amount on all sides is the same, and is based on the width
    /// of the map.
    pub fn expand(&self, factor: f64) -> Self {
        let mut bounds = self.clone();
        // expansion for each of the four bounding sides (width * factor / 2)
        let width_expansion = (bounds.ne.lng - bounds.sw.lng) * factor / 2.0;
        bounds.sw.lng -= width_expansion;
        bounds.sw.lat -= width_expansion;
        bounds.ne.lng += width_expansion;
        bounds.ne.lat += width_expansion;
        bounds
    }
}
