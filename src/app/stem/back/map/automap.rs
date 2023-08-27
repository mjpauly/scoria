//! Server route and generation logic for the automap, which occludes the
//! basemap wherever the user hasn't yet explored.
//!
//! Since it's updated in real-time, the tiles are given a short max-age in the
//! cache control header so the map library reloads it regularly.
//!
//! On-disk, unexplored regions are bincode encoded as geo::MultiPolygons which
//! correspond to map tiles in tile coordinates. The tile coordinates span 0-1
//! for each tile. When serving requests, the coordinates are scaled up to
//! 0-4096 in the protobuf.

use std::collections::HashSet;
use std::f64::consts::TAU;
use std::path::PathBuf;

use actix_web::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL};
use actix_web::{routes, web, HttpResponse, Responder};
use anyhow::anyhow;
use geo::{
    coord, line_string, AffineOps, AffineTransform, BooleanOps, Coord,
    CoordsIter, LineString, MapCoords, MapCoordsInPlace, MultiPolygon, Polygon,
    SimplifyVwPreserve, Winding,
};
use mvt::{Error, GeomEncoder, GeomType, Tile};
use pointy::Transform;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::app_state::AppState;
use crate::core::{debug, send_state_to_front};
use crate::database;
use crate::paths::get_unexplored_data_dir;
use common::filters::apply_filters;
use common::state::default_accuracy_filter;
use common::{LngLat, Location};

use super::coords::TileXYZ;

// Max supported zoom for the unexplored area
// If going beyond the maxzoom, the lower zoom levels can be overzoomed
static MAXZOOM: i32 = 15;

/// Determine if the automap is on or not. Defaults to false if the frontend
/// hasn't been initialized yet.
pub fn automap_is_on() -> bool {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front
        .as_ref()
        .map(|f| f.map.style.automap)
        .unwrap_or(false)
}

// ========== Tile Route and Encoding ========== //

#[routes]
#[get("/screen/{path:.*}")]
#[get("/analyze/screen/{path:.*}")]
async fn screen(path: web::Path<String>) -> impl Responder {
    let tile = match get_screen_tile(&path) {
        Some(t) => t,
        None => return HttpResponse::NotFound().finish(),
    };
    let mut reply = HttpResponse::Ok();
    reply.insert_header((
        actix_web::http::header::CONTENT_TYPE,
        "application/x-protobuf",
    ));
    // have screen tiles be re-requested every second so they're up-to-date
    reply.insert_header((CACHE_CONTROL, "max-age=2"));
    reply.insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
    reply.body(tile)
}

/// Return the tile bytes at the path
fn get_screen_tile(path: &web::Path<String>) -> Option<Vec<u8>> {
    if let Ok(tile) = TileXYZ::try_from(path) {
        let unexplored = if tile.z > MAXZOOM {
            // zoomed too far, just say we don't have it and have higher layers
            // get overzoomed
            return None;
        } else {
            match get_saved_screen(&tile) {
                SavedScreen::NotVisited => unexplored_tile_polygon(),
                SavedScreen::PartiallyExplored(saved) => saved,
                SavedScreen::FullyExplored => MultiPolygon::new(vec![]),
            }
        };
        // convert to vector tile bytes
        Some(polygon_to_tile(&unexplored).unwrap())
    } else {
        None
    }
}

static TILE_EXTENT_INT: u32 = 4096; // 4096 extent is standard
static TILE_EXTENT: f64 = TILE_EXTENT_INT as f64;

/// Convert the area polygons in tile coordinates (0–1) into the tile bytes.
///
/// Conforms to the 2.1 Mapbox Vector Tile Spec, available here:
/// https://github.com/mapbox/vector-tile-spec/tree/master/2.1/
fn polygon_to_tile(area: &MultiPolygon) -> Result<Vec<u8>, Error> {
    let mut tile = Tile::new(TILE_EXTENT_INT);
    add_layer_to_tile(&mut tile, "screen", area)?;
    tile.to_bytes()
}

/// Add a new layer to a tile with the given name and polygons.
fn add_layer_to_tile(
    tile: &mut Tile,
    name: &str,
    polygons: &MultiPolygon,
) -> Result<(), Error> {
    if polygons.iter().next().is_some() {
        let layer = tile.create_layer(name);
        let mut b = GeomEncoder::new(GeomType::Polygon, Transform::default());

        // when adding a point, normalize coordinates to the range 0–4096
        let add_point = |b: &mut GeomEncoder<f64>,
                         point: geo::geometry::Point| {
            b.add_point(point.x() * TILE_EXTENT, point.y() * TILE_EXTENT)
        };

        for polygon in polygons.iter() {
            // exterior must be clockwise per mvt spec
            let exterior = polygon.exterior().points_cw();
            // vector tile spec requires all linestrings to have nonzero length,
            // so we don't add the last point, which geo has already guaranteed
            // is the same as the starting point
            let n = exterior.len();
            for point in exterior.take(n - 1) {
                add_point(&mut b, point)?;
            }
            b.complete_geom()?;
            for interior in polygon.interiors() {
                // holes must be counter-clockwise
                let hole = interior.points_ccw();
                let n = hole.len();
                for point in hole.take(n - 1) {
                    add_point(&mut b, point)?;
                }
                b.complete_geom()?;
            }
        }

        // no idea why the API makes these steps are so roundabout:
        let feature = layer.into_feature(b.encode()?);
        let layer = feature.into_layer();
        tile.add_layer(layer)?;
    }
    Ok(())
}

// ========== Tile Updating ========== //

// how many records to examine at one time when updating the tiles. reduces file
// IO and difference operations for adjacent records (which are likely on the
// same tiles)
const BATCH_SIZE: u32 = 100;

// excludes multiple updaters from running at the same time
static UPDATE_LOCK: Mutex<()> = Mutex::const_new(());

pub async fn update_automap() {
    /*
     * HOTFIX: disabling automap update since geo difference seems to
     * occasionally get stuck in an infinite loop, causing 100% CPU usage, iOS
     * to shutdown the app, and location to not get logged.
     * TODO: switch to a different difference operation like in geo_clipper
    if automap_is_on() {
        // Prevent concurrent updating:
        let Ok(_update_guard) = UPDATE_LOCK.try_lock() else { return };

        debug("Updating automap");
        let mut last_automap_update = get_last_automap_update();
        update_num_automap_records_remaining(&last_automap_update).await;
        // get a batch of records to update the automap with
        let mut records_batch = get_records_batch(&last_automap_update).await;
        // while the retrieval gives us a non-empty vector of records
        while let Some(last_record) = records_batch.last() {
            debug(&format!("recs in batch: {}", records_batch.len()));
            debug(&format!("last update: {:?}", last_automap_update));
            // get the explored area for the batch and update the tiles
            let explored = get_explored_area(&records_batch).await;
            update_tiles(explored);
            debug("after update tiles");

            // set the last update time to the last record's timestamp
            last_automap_update = last_record.timestamp;
            set_last_automap_update(&last_automap_update);
            update_num_automap_records_remaining(&last_automap_update).await;
            // debug(&format!("Updated to {:?}", last_automap_update));

            // get the next batch of records
            records_batch = get_records_batch(&last_automap_update).await;
        }
        send_state_to_front();
        debug("Done updating automap");
    }
    */
}

async fn get_records_batch(
    last_update: &time::OffsetDateTime,
) -> Vec<common::Location> {
    database::get_records_after_with_limit(last_update, BATCH_SIZE).await
}

pub fn get_last_automap_update() -> time::OffsetDateTime {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .back
        .last_automap_update
        .0
}

fn set_last_automap_update(val: &time::OffsetDateTime) {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .back
        .last_automap_update
        .0 = *val;
}

/// Notifies the frontend how many records remain to show an indication of the
/// automap status.
async fn update_num_automap_records_remaining(
    last_update: &time::OffsetDateTime,
) {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .back
        .num_automap_records_remaining =
        database::count_records_since(*last_update).await as u64;
}

// radius of the earth (m)
const EARTH_RADIUS: f64 = 6.378e6;
// radius in meters of a point's explored area
const VIEW_RADIUS_METERS: f64 = 100.0;
// lat/lon radius for a point's view area (at equator)
const VIEW_RADIUS_DEG: f64 = (VIEW_RADIUS_METERS / EARTH_RADIUS) * 360.0 / TAU;

// minimum triangle area to keep during simplification. 1/1000 of a tile in each
// axis, which is greater than the 1/4096 fractional truncation grid size.
const MIN_KEEP_AREA: f64 = 0.001 * 0.001;

/// Get the area explored by a set of records (union of their view ellipses)
/// and return it in tile coordinates at the maximum zoom level.
async fn get_explored_area(
    unfiltered_records: &[common::Location],
) -> MultiPolygon {
    // debug("explored area");
    // filter out data points with accuracy worse than 100m (our view radius)
    let filtered_records =
        apply_filters(&default_accuracy_filter(), unfiltered_records);
    // debug("applied filters");
    let mut explored = MultiPolygon::new(vec![]);
    for (i, rec) in filtered_records.iter().enumerate() {
        // debug(&format!("rec {}", i));
        let new = MultiPolygon::new(vec![calc_explored_polygon(rec)]);
        // debug("calc'd explored");
        explored = improved_union(&explored, &new).unwrap_or(explored);
        // debug("unioned");
        explored = explored.simplify_vw_preserve(&MIN_KEEP_AREA);
        // debug("simplified");
        // println!("pts: {}", explored.coords_count());
    }
    explored
}

/// Get the explored area around a point, which is modeled as an ellipse in
/// latlng coordinates, and converted to tile coordinates at the max zoom and
/// franctionally truncated.
fn calc_explored_polygon(rec: &Location) -> Polygon {
    let n = 32;
    Polygon::new(
        LineString::new(
            (0..n)
                .map(|i| {
                    let theta = i as f64 * TAU / n as f64;
                    let tilecoords = TileXYZ::from_lnglat(
                        &LngLat {
                            // space between longitude lines is roughly a
                            // function of cos(latitude), so we divide by that
                            // factor to scale up the range of longitudes seen
                            lng: rec.longitude
                                + VIEW_RADIUS_DEG * theta.cos()
                                    / rec.latitude.to_radians().cos(),
                            lat: rec.latitude + VIEW_RADIUS_DEG * theta.sin(),
                        },
                        MAXZOOM,
                    );
                    Coord {
                        x: tilecoords.x,
                        y: tilecoords.y,
                    }
                })
                .collect(),
        ),
        vec![],
    )
}

/// Updates tiles at each zoom level. When moving to the next zoom level, all
/// coordinates are halved and the explored area is simplified again.
fn update_tiles(mut explored: MultiPolygon) {
    // debug("update_tiles");
    // debug(&format!("explored: {:?}", explored));
    // divide all coordinates by two when going to next zoom level
    let halve_coords = AffineTransform::scale(0.5, 0.5, coord! {x: 0., y: 0. });
    for z in (0..=MAXZOOM).rev() {
        // println!("zoom: {z}");
        // println!("z {}, pts: {}", z, explored.coords_count());
        explored = explored.simplify_vw_preserve(&MIN_KEEP_AREA);
        // println!("simplified");
        // println!("pts after tile simplify: {}", explored.coords_count());
        update_tiles_at_zoom(&explored, z);
        // println!("updated tiles");
        explored.affine_transform_mut(&halve_coords);
        // println!("halved coords");
    }
}

// amount of overlap between tiles (1%)
const MARGIN: f64 = 0.01;

fn update_tiles_at_zoom(explored: &MultiPolygon, z: i32) {
    let mut tiles_to_update = HashSet::new();
    for coord in explored.coords_iter() {
        // four corners where we move by MARGIN in X and Y captures the cases
        // where the point would be covered by another tile's overlap
        let cases = [
            (coord.x - MARGIN, coord.y - MARGIN),
            (coord.x + MARGIN, coord.y - MARGIN),
            (coord.x + MARGIN, coord.y + MARGIN),
            (coord.x - MARGIN, coord.y + MARGIN),
        ];
        // for each case, calculate tile XY, add to set
        for (x, y) in cases {
            tiles_to_update.insert((x.floor() as i32, y.floor() as i32));
        }
    }
    // for each tile, difference with explored and save back to file
    for (x, y) in tiles_to_update {
        // println!("xy: {x}, {y}");
        let x = x as f64;
        let y = y as f64;
        let tilepos = TileXYZ { x, y, z };
        let saved = match get_saved_screen(&tilepos) {
            SavedScreen::NotVisited => unexplored_tile_polygon(),
            SavedScreen::PartiallyExplored(saved) => saved,
            SavedScreen::FullyExplored => continue,
        };
        // println!("got saved");
        // translate the explored region into the normalized tile coordinates
        let shifted_explored =
            explored.affine_transform(&AffineTransform::translate(-x, -y));
        // println!("translated");
        if let Ok(new_unexplored) =
            improved_difference(&saved, &shifted_explored)
        {
            // println!("diff'ed");
            write_saved_screen(&tilepos, &new_unexplored);
            // println!("wrote");
        }
    }
}

// factor to scale up by, round, and scale back down
const TRUNCATE_RESOLUTION: f64 = 4096.0;

fn scale_up_and_round(point: Coord) -> Coord {
    Coord {
        x: (point.x * TRUNCATE_RESOLUTION).round(),
        y: (point.y * TRUNCATE_RESOLUTION).round(),
    }
}

fn scale_down(point: Coord) -> Coord {
    Coord {
        x: point.x / TRUNCATE_RESOLUTION,
        y: point.y / TRUNCATE_RESOLUTION,
    }
}

/// Somewhat reliably difference two MultiPolygons by scaling them up by some
/// resolution, and rounding all fractional values to integers, which are
/// precisely representable (within the range −2^53 to 2^53 for f64). The
/// difference operation is then much better behaved and encounters fewer
/// arthmetic errors, and we can scale the result back down to the original
/// size.
///
/// Occasionally, the operation still fails, in which case we catch the panic
/// emitted with std::panic::catch_unwind, and return an error.
///
/// Inputs should have feature sizes that are larger than the resolution,
/// othersize the truncation may produce invalid geometry. This is done by
/// removing any points that form triangles with area smaller than 1e-6 in area
/// with simplify_vw_preserve. With a resolution of 4096, only triangles with
/// area smaller than 6e-8 might have vertices that fall on the same point.
fn improved_difference(
    left: &MultiPolygon,
    right: &MultiPolygon,
) -> anyhow::Result<MultiPolygon> {
    let left = left.map_coords(scale_up_and_round);
    let right = right.map_coords(scale_up_and_round);
    // println!("about to diff");
    // println!("left: {:?}", left);
    // println!("right: {:?}", right);
    // std::fs::write("left.bin", bincode::serialize(&left).unwrap()).unwrap();
    // std::fs::write("right.bin", bincode::serialize(&right).unwrap()).unwrap();
    // println!("{:?}", std::env::current_dir().unwrap());
    let mut diff = std::panic::catch_unwind(|| left.difference(&right))
        .map_err(|_| {
            println!("right: {:?}", right);
            debug("Failed to compute polygon difference despite truncating.");
            anyhow!("Failed to compute difference")
        })?;
    // println!("diffed");
    diff.map_coords_in_place(scale_down);
    Ok(diff)
}

/// Same as improved_difference but for union.
fn improved_union(
    left: &MultiPolygon,
    right: &MultiPolygon,
) -> anyhow::Result<MultiPolygon> {
    let left = left.map_coords(scale_up_and_round);
    let right = right.map_coords(scale_up_and_round);
    let mut union =
        std::panic::catch_unwind(|| left.union(&right)).map_err(|_| {
            debug("Failed to compute polygon union despite truncating.");
            anyhow!("Failed to compute union")
        })?;
    union.map_coords_in_place(scale_down);
    Ok(union)
}

/// Generate a tile MultiPolygon which is fully unexplored. Coordinates are in
/// the 0-1 range, with the added margin overlap.
fn unexplored_tile_polygon() -> MultiPolygon {
    let coords = line_string![
        (x: 0.0 - MARGIN, y: 0.0 - MARGIN),
        (x: 1.0 + MARGIN, y: 0.0 - MARGIN),
        (x: 1.0 + MARGIN, y: 1.0 + MARGIN),
        (x: 0.0 - MARGIN, y: 1.0 + MARGIN)
    ];
    MultiPolygon::new(vec![Polygon::new(coords, vec![])])
}

/// State of the automap screen
#[derive(Clone, Debug, Serialize, Deserialize)]
enum SavedScreen {
    NotVisited,    // no file/data saved
    FullyExplored, // file is empty
    PartiallyExplored(MultiPolygon),
}

/// Determines if a given tile has been fully or partially explored yet.
pub fn tile_has_been_visited(tilepos: &TileXYZ) -> bool {
    let mut tilepos = tilepos.clone();
    while tilepos.z > MAXZOOM {
        // if we're zoomed too far in, determine visitation based on parent tile
        tilepos.x /= 2.;
        tilepos.y /= 2.;
        tilepos.z -= 1;
    }
    let path = automap_tilepos_to_path(&tilepos);
    std::fs::metadata(path).is_ok()
}

/// Retrieve a saved unexplored area if it exists.
fn get_saved_screen(tilepos: &TileXYZ) -> SavedScreen {
    let path = automap_tilepos_to_path(tilepos);
    match std::fs::metadata(path.clone()) {
        Ok(metadata) => {
            if metadata.len() == 0 {
                SavedScreen::FullyExplored
            } else {
                SavedScreen::PartiallyExplored(
                    bincode::deserialize(&std::fs::read(path).unwrap())
                        .unwrap(),
                )
            }
        }
        Err(_) => SavedScreen::NotVisited,
    }
}

/// Save a screen to file. Assumes some of the tile's region has been explored,
/// and so there is reason to write a file (if it's unexplored, the file should
/// not exist).
///
/// Contents are in Tile XY coordinates, in these ranges:
///     [-margin, 1 + margin]
///     [-margin, 1 + margin]
fn write_saved_screen(tilepos: &TileXYZ, polygons: &MultiPolygon) {
    let path = automap_tilepos_to_path(tilepos);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    if polygons.iter().next().is_none() {
        // fully explored (no polygons), write an empty file
        std::fs::write(path, b"").unwrap();
    } else {
        std::fs::write(path, bincode::serialize(&polygons).unwrap()).unwrap();
    }
}

fn automap_tilepos_to_path(tilepos: &TileXYZ) -> PathBuf {
    get_unexplored_data_dir().join(format!(
        "{}/{}/{}.bin",
        tilepos.z,
        tilepos.x.floor() as i32,
        tilepos.y.floor() as i32
    ))
}
