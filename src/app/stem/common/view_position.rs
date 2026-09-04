//! A position from which the map is being viewed.

use crate::LngLat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ViewPosition {
    pub center: LngLat,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
    pub bounds: LngLatBounds,
}

impl ViewPosition {
    /// The view bounds expanded on all sides by `px` screen pixels at this
    /// view's zoom (512 px world tile at zoom 0). The longitude pad is
    /// applied to latitude as well, which slightly over-pads it off the
    /// equator.
    pub fn expanded_bounds(&self, px: f64) -> LngLatBounds {
        let deg_per_px = 360.0 / (512.0 * 2_f64.powf(self.zoom));
        let pad = px * deg_per_px;
        let mut bounds = self.bounds;
        bounds.sw.lng -= pad;
        bounds.sw.lat -= pad;
        bounds.ne.lng += pad;
        bounds.ne.lat += pad;
        bounds
    }
}

/// A latitude/longitude bounding box, defined by its southwest and northeast
/// corners. In Maplibre, these corners can have longitude values greater than
/// 180 or less than -180 when the antimeridian is visible. When determining if
/// a point is contained inside the bounds we thus have to check wrapped
/// longitude values as well
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
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
}
