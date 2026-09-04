//! Web Mercator grid cells for the spatial decimation query, and the
//! gx/gy backfill for rows that predate the grid migration
//! (migrations/20260827000000_grid_cells.sql) or arrived through an import.
//!
//! A point's grid coordinates (gx, gy) are u32 positions over the world; a
//! cell at level L is the 0.5 CSS px cell at view zoom L, `g >> (22 - L)`.
//! Indexed levels get a (cy, cx, timestamp) index so "the newest point per
//! cell in view" is a loose index scan over occupied cells rather than a
//! scan over every point in view (doc/decimation/probe-results/
//! cell-index-levels.md for the measurements behind the level choice).

use anyhow::{Context, Result};
use common::LngLat;
use sqlx::{QueryBuilder, SqlitePool};
use tracing::info;

/// Grid resolution: 2^32 divisions per axis (~9 mm at the equator).
pub const GRID_BITS: u32 = 32;
/// Cell levels with a (cy, cx, timestamp) index, coarsest first. Must match
/// the generated columns in the migration.
pub const INDEX_LEVELS: [u8; 3] = [8, 10, 12];
/// Largest representable Mercator latitude.
const MAX_LAT: f64 = 85.051_128_78;

/// Per-axis shift from grid coordinates to the cell at `level` (0.5 px at
/// view zoom `level`, 512 px tiles: 2^(level + 10) divisions per axis).
pub fn level_shift(level: u8) -> u32 {
    GRID_BITS - 10 - level as u32
}

pub fn index_name(level: u8) -> String {
    format!("location_cell{level}")
}

/// Grid coordinates of a point. Latitude is clamped to the Mercator range and
/// longitude wrapped into [-180, 180).
pub fn grid_coords(lnglat: &LngLat) -> (u32, u32) {
    let lat = lnglat.lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let lng = (lnglat.lng + 180.).rem_euclid(360.) - 180.;
    let x = (lng + 180.) / 360.;
    let y = (std::f64::consts::PI
        - (std::f64::consts::FRAC_PI_4 + lat / 2.).tan().ln())
        / std::f64::consts::TAU;
    let scale = 2_f64.powi(GRID_BITS as i32);
    let to_u32 = |v: f64| (v * scale).floor().clamp(0., u32::MAX as f64) as u32;
    (to_u32(x), to_u32(y))
}

/// Per-axis shift of the query cell for a cell pitch in CSS px at a view
/// zoom. Cells are grid-aligned powers of two, so the pitch is quantized;
/// the finer power of two is chosen so the result never has fewer points
/// than the requested pitch would give.
pub fn query_shift(zoom: f64, pitch_px: f64) -> u32 {
    // grid units per CSS px at this zoom: 2^32 / (512 * 2^zoom)
    let units = (GRID_BITS as f64 - 9. - zoom + pitch_px.log2()).floor();
    units.clamp(0., (GRID_BITS - 1) as f64) as u32
}

/// Which index serves a query cell of the given shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexUse {
    /// The index cell is the query cell or finer: walk occupied index cells
    /// and take the newest point per query cell.
    Walk { level: u8 },
    /// The query cell is finer than every index: the coarsest index only
    /// serves as a 2-D bounds filter and the grouping reads row coordinates.
    /// The coarsest is fastest here because its per-cell timestamp order
    /// reads table rows in long sequential runs.
    Filter { level: u8 },
}

pub fn index_for(query_shift: u32) -> IndexUse {
    // the coarsest index whose cell is not coarser than the query cell,
    // i.e. the first (smallest) level with shift <= query shift
    INDEX_LEVELS
        .iter()
        .find(|&&l| level_shift(l) <= query_shift)
        .map(|&level| IndexUse::Walk { level })
        .unwrap_or(IndexUse::Filter {
            level: INDEX_LEVELS[0],
        })
}

/// Index level for a 2-D bounds filter over a rectangle `w` x `h` grid
/// units (the temporal-mode count and fetch, which have no query cell).
/// The coarsest level whose cells span the rectangle's smaller side at
/// least `MIN_CELLS_ACROSS` times: coarser cells mean fewer per-row seeks,
/// while too-coarse cells put most of the rectangle in edge cells, whose
/// rows must be read to check exact bounds. Measured in
/// doc/decimation/probe-results/time-range-bench.md.
pub fn filter_level(w: u32, h: u32) -> u8 {
    const MIN_CELLS_ACROSS: u32 = 16;
    INDEX_LEVELS
        .iter()
        .copied()
        .find(|&l| {
            let s = level_shift(l);
            (w >> s).min(h >> s) + 1 >= MIN_CELLS_ACROSS
        })
        .unwrap_or(INDEX_LEVELS[INDEX_LEVELS.len() - 1])
}

/// Fill gx/gy where NULL, walking rowids upward from the first NULL row.
/// The NULL probes test cy8 (NULL exactly when gy is), which is the leading
/// column of location_cell8, so once that index exists an open with nothing
/// to fill is a seek rather than a table scan.
/// Each batch is one statement, the batch's rows as a VALUES table joined
/// to location by id:
///
/// ```sql
/// WITH v(id, gx, gy) AS (VALUES (?, ?, ?), (?, ?, ?), ...)
/// UPDATE location SET gx = v.gx, gy = v.gy FROM v WHERE location.id = v.id
/// ```
///
/// so the 4000 rows cost one round trip through sqlx rather than 4000
/// (3 binds per row stays under SQLite's variable limit (32766) at this batch
/// size; see `push_bind()`). Each batch is its own implicit transaction, so a
/// kill mid-way resumes where it left off.
/// `progress` is called with (rows filled, rows to fill) after each batch.
/// Run by open_db before the level indexes exist, and after imports.
pub async fn backfill(
    conn: &SqlitePool,
    mut progress: impl FnMut(u64, u64),
) -> Result<u64> {
    let started = std::time::Instant::now();
    let Some(mut last_id) = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT min(id) FROM location WHERE cy8 IS NULL",
    )
    .fetch_one(conn)
    .await
    .context("finding first row to backfill")?
    else {
        return Ok(0);
    };
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM location WHERE cy8 IS NULL",
    )
    .fetch_one(conn)
    .await
    .context("counting rows to backfill")? as u64;
    last_id -= 1;
    let mut filled = 0u64;
    loop {
        let rows = sqlx::query_as::<_, (i64, f64, f64)>(
            "SELECT id, latitude, longitude FROM location WHERE id > ? AND \
             cy8 IS NULL ORDER BY id LIMIT ?",
        )
        .bind(last_id)
        .bind(BATCH)
        .fetch_all(conn)
        .await?;
        if rows.is_empty() {
            break;
        }
        last_id = rows.last().unwrap().0;
        let mut q = QueryBuilder::new("WITH v(id, gx, gy) AS (");
        q.push_values(&rows, |mut b, (id, lat, lng)| {
            let (gx, gy) = grid_coords(&LngLat {
                lng: *lng,
                lat: *lat,
            });
            b.push_bind(id).push_bind(gx as i64).push_bind(gy as i64);
        });
        q.push(
            ") UPDATE location SET gx = v.gx, gy = v.gy FROM v WHERE \
             location.id = v.id",
        );
        q.build().execute(conn).await?;
        filled += rows.len() as u64;
        progress(filled, total);
    }
    info!("grid backfill: {filled} rows, {:.1?}", started.elapsed());
    Ok(filled)
}

const BATCH: i64 = 4_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coords_and_shifts() {
        // origin of the grid is the north-west corner
        let (x, y) = grid_coords(&LngLat {
            lng: -180.,
            lat: 85.,
        });
        assert_eq!(x, 0);
        assert!(y < 1 << 24);
        let (x, y) = grid_coords(&LngLat { lng: 0., lat: 0. });
        assert_eq!(x, 1 << 31);
        assert_eq!(y, 1 << 31);
        // longitude wraps, latitude clamps
        let (x, _) = grid_coords(&LngLat { lng: 190., lat: 0. });
        assert_eq!(
            x,
            grid_coords(&LngLat {
                lng: -170.,
                lat: 0.
            })
            .0
        );
        let (_, y) = grid_coords(&LngLat { lng: 0., lat: -89. });
        assert_eq!(
            y,
            grid_coords(&LngLat {
                lng: 0.,
                lat: -MAX_LAT
            })
            .1
        );

        // 0.5 px at zoom L is exactly level L's cell
        for l in INDEX_LEVELS {
            assert_eq!(query_shift(l as f64, 0.5), level_shift(l));
        }
        // fractional zoom rounds to the finer cell; coarser pitch coarsens
        assert_eq!(query_shift(10.3, 0.5), level_shift(10) - 1);
        assert_eq!(query_shift(10., 2.), level_shift(10) + 2);

        assert_eq!(index_for(level_shift(10)), IndexUse::Walk { level: 10 });
        assert_eq!(index_for(level_shift(9)), IndexUse::Walk { level: 10 });
        assert_eq!(index_for(level_shift(6)), IndexUse::Walk { level: 8 });
        assert_eq!(index_for(level_shift(13)), IndexUse::Filter { level: 8 });

        // wide rectangles take the coarsest level, narrow ones the finest
        assert_eq!(filter_level(u32::MAX, u32::MAX), 8);
        assert_eq!(filter_level(64 << level_shift(8), 32 << level_shift(8)), 8);
        assert_eq!(filter_level(8 << level_shift(8), 64 << level_shift(8)), 10);
        assert_eq!(filter_level(1 << level_shift(8), 64 << level_shift(8)), 12);
        assert_eq!(filter_level(0, 0), 12);
    }
}
