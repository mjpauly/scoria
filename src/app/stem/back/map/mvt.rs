//! Serves the current map query result as Mapbox vector tiles, sliced on
//! demand from a per-mount working set: the browser then only holds the
//! visible tiles instead of a client-side tile pyramid over the whole
//! result (doc/decimation/memory-limits.md, "MVT tile serving").

use std::sync::Arc;

use actix_identity::Identity;
use actix_web::{routes, web, HttpResponse, Responder};
use common::map_style::{MAP_MAXZOOM, MVT_SOURCE_MAXZOOM as SOURCE_MAXZOOM};
use common::mounted::MountID;
use mvt::{GeomEncoder, GeomType, Tile};
use pointy::Transform;

use crate::app_state::AppState;
use crate::server::no_caching_directives;

const TILE_EXTENT_INT: u32 = 4096; // 4096 extent is standard
const TILE_EXTENT: f64 = TILE_EXTENT_INT as f64;

/// The current query result in z0 web-mercator coordinates (0-1 across the
/// world), ready to slice into tiles.
#[derive(Debug, Default)]
pub struct TileSet {
    pub points: Vec<TilePoint>,
    pub segs: Vec<TileSeg>,
}

/// A point of the working set. Colors are cmap palette entries, so they
/// borrow rather than allocate.
#[derive(Debug)]
pub struct TilePoint {
    pub x: f64,
    pub y: f64,
    pub color: Option<&'static str>,
}

/// A line segment of the working set.
#[derive(Debug)]
pub struct TileSeg {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: Option<&'static str>,
}

impl TileSet {
    /// Approximate heap bytes of the working set, the N-scaling backend
    /// memory cost of the query result.
    pub fn approx_bytes(&self) -> usize {
        use std::mem::size_of;
        self.points.capacity() * size_of::<TilePoint>()
            + self.segs.capacity() * size_of::<TileSeg>()
    }
}

/// Store the working set built by make_tileset for a mount.
pub async fn store_tileset(mount_id: MountID, tileset: TileSet) {
    AppState::global()
        .map_data
        .mount_tilesets
        .write()
        .await
        .insert(mount_id, Arc::new(tileset));
}

/// Evict a mount's working set (on unmount).
pub async fn remove_tileset(mount_id: MountID) {
    AppState::global()
        .map_data
        .mount_tilesets
        .write()
        .await
        .remove(&mount_id);
}

/// Evict working sets of mounts not in `keep` (disabled mounts).
pub async fn retain_tilesets(keep: &[MountID]) {
    AppState::global()
        .map_data
        .mount_tilesets
        .write()
        .await
        .retain(|id, _| keep.contains(id));
}

/// Drop all working sets to reduce idle memory (on app backgrounding).
/// Rebuilt by the foregrounding update_map_data pass.
pub async fn clear_tilesets() {
    AppState::global()
        .map_data
        .mount_tilesets
        .write()
        .await
        .clear();
}

/// Tile route. Responses are no-store and sliced from the mount's current
/// working set at request time, so query updates need no cache-busting;
/// the front refreshes by reloading tiles in maplibre's "expired" state.
#[routes]
#[get("/{mount_id}/tiles/{z}/{x}/{y}.mvt")]
#[get("/analyze/{mount_id}/tiles/{z}/{x}/{y}.mvt")]
pub async fn mvt_route(
    _: Identity,
    path: web::Path<(MountID, i32, u32, u32)>,
) -> impl Responder {
    let (mount_id, z, x, y) = path.into_inner();
    // Clone the Arc out so the map lock is held only for the lookup, and
    // run the CPU-bound slice on the blocking pool; concurrent tile
    // requests then slice in parallel off the async executor threads.
    let app_state = AppState::global();
    let tileset = {
        let tilesets = app_state.map_data.mount_tilesets.read().await;
        tilesets.get(&mount_id).cloned()
    };
    let body = match tileset {
        Some(ts) => web::block(move || slice_tile(&ts, z, x, y))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()))
            .unwrap_or_else(|e| {
                tracing::error!("slicing tile {z}/{x}/{y}: {e}");
                Vec::new()
            }),
        None => Vec::new(),
    };
    HttpResponse::Ok()
        .content_type("application/x-protobuf")
        .insert_header(no_caching_directives())
        .body(body)
}

/// Encode the tile at z/x/y from the working set. One feature per point /
/// segment with its color tag (unmerged; run merging can come later if the
/// per-feature overhead shows up in the probe).
fn slice_tile(
    ts: &TileSet,
    z: i32,
    x: u32,
    y: u32,
) -> Result<Vec<u8>, mvt::Error> {
    let scale = 2_f64.powi(z);
    let (tx, ty) = (x as f64, y as f64);
    let mut tile = Tile::new(TILE_EXTENT_INT);

    // Points: those within a buffer of the tile, so a marker centered just
    // outside still paints its overflow in this tile. The patched maplibre
    // (front/patch_maplibre.sh) stencil-clips circles to their tile, so the
    // buffered copies are not double drawn and each pixel gets its tile's
    // feature order, i.e. the global order of ts.points.
    let buffer = point_buffer(z);
    let (lo, hi) = (-buffer, 1.0 + buffer);
    let mut layer = tile.create_layer("points");
    let mut any = false;
    for p in &ts.points {
        let lx = p.x * scale - tx;
        let ly = p.y * scale - ty;
        if !(lo..hi).contains(&lx) || !(lo..hi).contains(&ly) {
            continue;
        }
        let mut b = GeomEncoder::new(GeomType::Point, Transform::default());
        b.add_point(lx * TILE_EXTENT, ly * TILE_EXTENT)?;
        let mut feature = layer.into_feature(b.encode()?);
        if let Some(c) = p.color {
            feature.add_tag_string("color", c);
        }
        layer = feature.into_layer();
        any = true;
    }
    if any {
        tile.add_layer(layer)?;
    }

    // Segments: clipped to the tile plus a buffer. Maplibre stencil-clips
    // line rendering per tile, but its geometry loader also clamps
    // coordinates to +-4 tile widths, which bends any segment with a far
    // off-tile endpoint (visible as slope breaks at tile boundaries when
    // zoomed in). The buffer keeps the stencil clip, not the clipped end,
    // as the visible edge.
    let mut layer = tile.create_layer("lines");
    let mut any = false;
    for s in &ts.segs {
        let x1 = s.x1 * scale - tx;
        let y1 = s.y1 * scale - ty;
        let x2 = s.x2 * scale - tx;
        let y2 = s.y2 * scale - ty;
        let Some((x1, y1, x2, y2)) = clip_segment(x1, y1, x2, y2) else {
            continue;
        };
        let mut b =
            GeomEncoder::new(GeomType::Linestring, Transform::default());
        b.add_point(x1 * TILE_EXTENT, y1 * TILE_EXTENT)?;
        b.add_point(x2 * TILE_EXTENT, y2 * TILE_EXTENT)?;
        let mut feature = layer.into_feature(b.encode()?);
        if let Some(c) = s.color {
            feature.add_tag_string("color", c);
        }
        layer = feature.into_layer();
        any = true;
    }
    if any {
        tile.add_layer(layer)?;
    }

    tile.to_bytes()
}

/// Tile buffer for points, in tile widths: the largest drawn radius (marker
/// plus its invisible click stroke, which pads markers out to the front's
/// CLICK_TARGET_PX), in pixels of a 512 px tile, scaled by the overzoom a
/// tile at zoom z can be shown at.
fn point_buffer(z: i32) -> f64 {
    const MAX_MARKER_PX: f64 = common::map_style::MARKER_SIZE_MAX;
    const CLICK_TARGET_PX: f64 = 10.0;
    // A tile shows from zoom z to just under z+1, or to the map's max zoom
    // for tiles at the source maxzoom.
    let overzoom_levels = if z >= SOURCE_MAXZOOM {
        MAP_MAXZOOM - z
    } else {
        1
    };
    let overzoom = 2_f64.powi(overzoom_levels.max(0));
    MAX_MARKER_PX.max(CLICK_TARGET_PX) / 512.0 * overzoom
}

/// Tile buffer for line clipping, in tile widths (256/4096, maplibre's
/// default). Must exceed half the widest line so the tile edge, not the
/// clipped end, bounds what's drawn.
const LINE_CLIP_BUFFER: f64 = 256.0 / TILE_EXTENT;

/// Liang-Barsky clip of a segment (in tile-normalized coordinates) to the
/// tile plus LINE_CLIP_BUFFER. None if the segment misses the tile.
fn clip_segment(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Option<(f64, f64, f64, f64)> {
    let (lo, hi) = (-LINE_CLIP_BUFFER, 1.0 + LINE_CLIP_BUFFER);
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    // (p, q) per edge: the segment is inside for t where p*t <= q
    for (p, q) in [(-dx, x1 - lo), (dx, hi - x1), (-dy, y1 - lo), (dy, hi - y1)]
    {
        if p == 0.0 {
            if q < 0.0 {
                return None; // parallel and outside
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((x1 + t0 * dx, y1 + t0 * dy, x1 + t1 * dx, y1 + t1 * dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_slope_and_drops_misses() {
        // crosses the tile diagonally from far away: endpoints move onto
        // the buffer box, slope preserved
        let (x1, y1, x2, y2) = clip_segment(-10.0, -5.0, 10.0, 5.0).unwrap();
        assert!((x1 + LINE_CLIP_BUFFER).abs() < 1e-12);
        assert!(((y2 - y1) / (x2 - x1) - 0.5).abs() < 1e-12);
        assert!(x2 <= 1.0 + LINE_CLIP_BUFFER + 1e-12);
        // fully inside: untouched
        assert_eq!(
            clip_segment(0.2, 0.3, 0.4, 0.5),
            Some((0.2, 0.3, 0.4, 0.5))
        );
        // misses: bbox overlaps the tile but the line passes the corner
        assert_eq!(clip_segment(-1.0, 0.5, 0.5, 2.0), None);
        // misses: entirely to one side, axis-parallel
        assert_eq!(clip_segment(2.0, 0.0, 2.0, 1.0), None);
    }
}
