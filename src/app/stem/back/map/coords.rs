//! Utilities for converting between tile coordinates and lnglat in the Web
//! Mercator Projection, as well as parsing the tile coordinates from a web
//! server route.
//!
//! See https://en.wikipedia.org/wiki/Web_Mercator_projection for the
//! formulas (lnglat to tile, the other way must be derived).

use std::f64::consts::TAU;

use actix_web::{
    dev::{Path, ResourceDef},
    web,
};
use anyhow::{anyhow, Error, Result};

use common::LngLat;

#[derive(Clone, Debug)]
pub struct TileXYZ {
    pub x: f64,
    pub y: f64,
    pub z: i32,
}

impl TileXYZ {
    /// Convert integer tile coordinates to floating point lnglat in degrees.
    pub fn to_lnglat(&self) -> LngLat {
        let coeff = TAU / 2_f64.powi(self.z);
        let lng = coeff * self.x - TAU / 2.0;
        let lat = 2. * (TAU / 2. - coeff * self.y).exp().atan() - TAU / 4.;
        LngLat {
            lng: lng.to_degrees(),
            lat: lat.to_degrees(),
        }
    }

    /// Convert floating point lnglat in degrees to the integer tile coordinates
    /// of the tile that the point would fall into.
    pub fn from_lnglat(lnglat: &LngLat, z: i32) -> TileXYZ {
        let coeff = 1. / TAU * 2_f64.powi(z);
        let x = coeff * (lnglat.lng.to_radians() + TAU / 2.);
        let y = coeff
            * (TAU / 2. - (TAU / 8. + lnglat.lat.to_radians() / 2.).tan().ln());
        TileXYZ { x, y, z }
    }
}

impl TryFrom<&web::Path<String>> for TileXYZ {
    type Error = Error;

    /// Parse tile coordinates from a web server route in the standard format.
    fn try_from(orig: &web::Path<String>) -> Result<Self> {
        let mut path = Path::new(orig.as_str());
        if ResourceDef::new("tiles/{tileset}/{z}/{x}/{y}.{ext}")
            .capture_match_info(&mut path)
        {
            let x = path.get("x").ok_or_else(|| anyhow!("No x."))?.parse()?;
            let y = path.get("y").ok_or_else(|| anyhow!("No y."))?.parse()?;
            let z = path.get("z").ok_or_else(|| anyhow!("No z."))?.parse()?;
            Ok(TileXYZ { x, y, z })
        } else {
            Err(anyhow!("Could not match tile path."))
        }
    }
}
