//! Handles database access and modification.
//!
//! # Schema
//!
//! Current schema, for quick reference:
//!
//! CREATE TABLE tmplocation
//! (
//!     id                          INTEGER NOT NULL PRIMARY KEY,
//!     timestamp                   INTEGER NOT NULL UNIQUE ON CONFLICT IGNORE,
//!
//!     latitude                    REAL    NOT NULL,
//!     longitude                   REAL    NOT NULL,
//!     horizontal_accuracy         REAL    NOT NULL,
//!
//!     msl_altitude                REAL,
//!     ellipsoid_altitude          REAL,
//!     vertical_accuracy           REAL,
//!     story                       INTEGER,
//!
//!     speed                       REAL,
//!     speed_accuracy              REAL,
//!
//!     course                      REAL,
//!     course_accuracy             REAL,
//!
//!     is_simulated_by_software    INTEGER,
//!     is_produced_by_accessory    INTEGER,
//!     was_imported                INTEGER NOT NULL DEFAULT 0
//! ) STRICT;
//!
//! With "STRICT" SQLite will ensure that we only insert the correct type into
//! each column. However, it is still possible to compare mismatched types in
//! our queries, so we must be careful there as well. When selecting rows in a
//! particular range, make sure to use the SQLite unixepoch() function and not
//! date() or datetime(), which return strings.
//!
//! We also use the unix epoch as the timestamp because comparisons are simpler.
//! sqlx/time format timestamps as "2023-03-15T05:15:56Z" by default but
//! SQLite's datetime() function will return "2023-03-15 05:15:56". This means
//! that date comparisons work fine, but anything requiring higher precision may
//! fail to compare correctly, since "T" always compares greater than " ". Using
//! seconds since the unix epoch is less error prone.
//!
//! The grid migration (migrations/20260827000000_grid_cells.sql) adds
//! Web Mercator grid coordinates gx/gy and virtual cell columns
//! cy{L}/cx{L} = gy/gx >> (22 - L) for L in 8, 10, 12 (database/grid.rs).
//!
//! # Interface
//!
//! As explained in the schema section, times stored in the database are stored
//! as their unix epoch. When returning data to callers, the LocationRow is
//! converted to a common::Location where the time is encoded as a
//! time::OffsetDateTime.
//!
//! # Map query routing
//!
//! Every map update builds one FilteredQuery per mounted database: bounds
//! (the expanded viewport), an optional time range, optional datastream
//! filters, a decimation threshold, the hard cap, and in spatial mode a
//! SpatialCell.
//! fetch_decimated_result_with_db turns that into records; the rest of this
//! section is about which index does the work. Measurements behind the
//! choices: doc/decimation/probe-results/time-range-bench.md.
//!
//! ## Access paths
//!
//! - rowid (id): the table itself, in insertion order, which is time order
//!   except after an out-of-order import. A scan in rowid order is the
//!   fastest way to read a large fraction of the table.
//! - The timestamp autoindex (from UNIQUE): entries (timestamp, rowid), so
//!   a time range is one contiguous run, and reading its rows is sequential
//!   in the table too.
//! - The grid level indexes location_cell{8,10,12} on (cy, cx, timestamp),
//!   0.5 px cells at view zoom 8/10/12, entries carrying the rowid. A
//!   rectangle in view is a set of cy rows, each a contiguous cx run.
//!
//! lngtimeindex (longitude, timestamp) was the spatial path before the
//! grid: a longitude band across all latitudes with a row read per entry
//! for the latitude check. On every shape measured it was within ~30% of
//! the grid or slower, so the grid_indexes migration drops it.
//!
//! The rule behind everything: a route's cost is proportional to the index
//! entries it walks, plus a row read for each entry that survives the
//! index-only predicates. Pick the route that walks the fewest.
//!
//! ## Common inputs (route_inputs)
//!
//! rows_in_range, counted on the timestamp index with `LIMIT cap + 1` so
//! it costs at most `cap` entries; None when the range is open on both
//! ends. A count above the cap means "large, unknown". The
//! cap is TIME_PATH_MAX_ROWS (300k) in spatial mode and TEMPORAL_COUNT_CAP
//! (1M) in temporal mode.
//!
//! TimeHint: the time bounds used to be wrapped in likelihood(X, 1.0) so
//! the planner would stay off the timestamp index. That is now a
//! parameter: UseIndex emits bare comparisons (the planner takes the
//! timestamp index), AvoidIndex keeps the hint. Grid queries pin their
//! index with INDEXED BY, so the hint matters only for the time route.
//!
//! ## Temporal mode (fetch_temporal)
//!
//! Temporal decimation counts the matching points to pick decim, then
//! fetches every decim-th by `id % decim`. Both enumerate everything in
//! the view and range. Two routes:
//!
//! - Time (fetch_temporal_time): count_query and bounded_decim_query with
//!   UseIndex. Walks the timestamp range, reads rows in rowid order,
//!   applies bounds, filters and the modulo. Cost ~ rows in range,
//!   sequential. With no range it is a table scan, which is right when the
//!   view holds most of the data.
//! - Grid (fetch_temporal_grid): fetch_count_grid then fetch_bounded_grid,
//!   both on grid_filter_query. Cost ~ all-time points in the view
//!   rectangle, index-only except edge cells; the time range and
//!   datastream filters apply afterwards and don't reduce what is walked.
//!
//! Decision:
//!   n_time = rows_in_range (over cap or None => cap + 1)
//!   n_view = fetch_count_grid(LIMIT min(n_time, cap), no filters)
//!   Grid if n_view < n_time; Grid if both capped and no range; else Time.
//!
//! n_view is bounded by n_time, so the estimates cost at most twice the
//! cheaper route's own scan. The view count is deliberately unfiltered:
//! the grid route walks every index entry in view whatever the range, so
//! its cost is the all-time count. A view-within-range count would be a
//! subset of rows-in-range, never exceed it, and pick Grid even on narrow
//! views where it is 20x slower. Ties go to Time because its scan is
//! sequential, and once both counts are capped the difference is a
//! constant factor that favours the sequential scan when a range is set
//! (275 ms vs 591 fully zoomed out). Both capped with no range means over
//! a million points in view and the alternative is an unindexed scan, so
//! Grid. The 1M cap: index-only counts cost ~15-30 ms per million entries,
//! affordable against 150-300 ms queries, and high enough that a narrow
//! view with a huge range still shows its 4x difference.
//!
//! grid_filter_query: grid::filter_level picks the coarsest level whose
//! cells span the rectangle's smaller side at least 16 times, else L12
//! (coarser means fewer cy rows to seek; too coarse and most of the
//! rectangle is edge cells whose rows must be read). A recursive CTE
//! generates the cy rows and joins the level index one row at a time, a
//! seek per row rather than a scan of the cy band. Interior cells are
//! accepted from the index; edge cells read the row for exact gx/gy
//! bounds, so the count is exact at every zoom (an L8 cell at z22 is the
//! whole screen; without the edge check the count would be up to 8x high
//! and decim too coarse in dense clusters). Being index-only, the count
//! needs no `id % 10` sampling.
//!
//! ## Spatial mode (fetch_spatial)
//!
//! Keeps the newest point per screen cell once the view is over the
//! bucketing trigger (min(threshold, hard cap)); under it every point is
//! returned as-is. Two routes for the bucketing:
//!
//! - Time: fetch_bucketed(shift, UseIndex), a GROUP BY on
//!   gy/gx >> shift over the rows in range with bounds and filters. Cost ~
//!   rows in range, ~150-200 ns each.
//! - Grid walk (fetch_bucketed_grid / grid_bucketed_query): the loose
//!   index scan, seeking from occupied index cell to occupied index cell
//!   in the rectangle, one more seek per cell for its newest passing row,
//!   grouped by query cell. Cost ~ 3-4 us per occupied index cell in view,
//!   independent of the time range. For query cells finer than L12 (zoom
//!   above ~13) the Filter form uses L8 as a bounds prefilter with exact
//!   gx/gy bounds and groups on row coordinates.
//!
//! Decision: Time if rows_in_range <= TIME_PATH_MAX_ROWS, else the walk.
//! A threshold rather than a count comparison because the walk's cost is
//! occupied cells in view, and the only way to count those is the walk;
//! points in view is an upper bound that is loose exactly in dense
//! clusters. The measured crossover is ~700-800k rows at z6-z13 and
//! ~150k at z16, so 300k is within 2x of optimal everywhere, and near the
//! crossover both paths cost ~100-150 ms. It is biased low because the
//! time path's cost is capped by the threshold while the walk's worst
//! case is the full all-time cost.
//!
//! The trigger check and the under-trigger fetch follow the same route:
//! timestamp index on Time; fetch_count_grid(LIMIT trigger + 1, with
//! filters) and fetch_bounded_grid(1) on Grid.
//!
//! ## Fallbacks
//!
//! The grid always exists by the time a query can run (open_db backfills
//! and builds the indexes before the pool is handed out). A grid query
//! error falls back to the time route rather than failing the map update.
//!
//! Both modes ask which index walks fewer entries, but only temporal mode
//! can answer by counting, because its grid cost (points in view) is
//! countable index-only. Spatial mode's grid cost (occupied cells) is not,
//! so it uses the one quantity it can measure cheaply, rows in range,
//! against a constant from the sweep.
//!

use std::path::PathBuf;

use anyhow::{Context, Result};
use common::{
    filters::{DataStream, Filter, FilterOp},
    mounted::{MountID, MAIN_DB_MOUNT_ID},
    popups::{PopUp, PopUpCode, PopUpKind},
    state::LastAutomapUpdate,
    view_position::LngLatBounds,
    LngLat, TimeRange, ToFront,
};
use jiff::Timestamp;
use sqlx::{
    migrate::Migrator, sqlite::SqliteRow, FromRow, QueryBuilder, Row, Sqlite,
    SqlitePool,
};
use tracing::{debug, error, info};

use crate::{
    app_state::{get_derived_state, get_front_state, AppState},
    database::{grid, mounted::db_debug_name, pins::update_derived_pins},
    logs::LogErrorAndContinue,
    map::map_data::update_map_data,
    runtime::get_runtime,
    ws_session::{send_error_popup, send_message_to_front, send_success_popup},
};

// Embed our migrations from "migrations/" into our binary at compile time
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// Struct representation of a Location row in the table
#[derive(Clone, FromRow, Debug)]
pub struct LocationRow {
    pub id: i64,
    pub timestamp: i64,

    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,

    pub msl_altitude: Option<f64>,
    pub ellipsoid_altitude: Option<f64>,
    pub vertical_accuracy: Option<f64>,
    pub story: Option<i64>,

    pub speed: Option<f64>,
    pub speed_accuracy: Option<f64>,

    pub course: Option<f64>,
    pub course_accuracy: Option<f64>,

    pub is_simulated_by_software: Option<bool>,
    pub is_produced_by_accessory: Option<bool>,

    pub was_imported: bool,
}

/// Narrow row returned by the decimated multi-row fetches: only what the
/// map and metrics paths read, so large fetches skip most of the
/// per-column decode cost (doc/decimation/probe-results/
/// grid-walk-alternatives.md, "Select width"). field1/field2 hold the
/// caller-chosen extra columns (FilteredQuery::field1/field2), CAST to
/// REAL; the caller keeps track of which column each holds. Full rows
/// for a narrow point come from fetch_full_locations or get_location_at
/// via the UNIQUE timestamp.
#[derive(Clone, Debug, PartialEq)]
pub struct NarrowPoint {
    pub id: i64,
    pub timestamp: time::OffsetDateTime,
    pub latitude: f64,
    pub longitude: f64,
    pub field1: Option<f64>,
    pub field2: Option<f64>,
}

impl NarrowPoint {
    pub fn lnglat(&self) -> LngLat {
        LngLat {
            lng: self.longitude,
            lat: self.latitude,
        }
    }
}

impl FromRow<'_, SqliteRow> for NarrowPoint {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let ts: i64 = row.try_get("timestamp")?;
        Ok(Self {
            id: row.try_get("id")?,
            timestamp: time::OffsetDateTime::from_unix_timestamp(ts).map_err(
                |e| sqlx::Error::ColumnDecode {
                    index: "timestamp".into(),
                    source: e.into(),
                },
            )?,
            latitude: row.try_get("latitude")?,
            longitude: row.try_get("longitude")?,
            field1: row.try_get("field1")?,
            field2: row.try_get("field2")?,
        })
    }
}

/// C FFI struct definition with special ways of encoding unavailable data
#[repr(C)]
#[derive(Clone, Debug)]
pub struct OSLocationData {
    // always available fields
    pub timestamp: i64,

    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,

    // altitudes valid only if vertical_accuracy >= 0
    pub msl_altitude: f64,
    pub ellipsoid_altitude: f64,
    pub vertical_accuracy: f64,

    // bool indivates if story data is available
    pub story_available: bool,
    pub story: i64,

    // marked as unavailable with -1
    pub speed: f64,
    pub speed_accuracy: f64,
    pub course: f64,
    pub course_accuracy: f64,

    // indicates if source info is available, or should be NULL
    pub source_info_available: bool,
    pub is_simulated_by_software: bool,
    pub is_produced_by_accessory: bool,
}

/// common::Location is about the representation needed for inserting into the
/// database, so we use this parser for log_location()
impl std::convert::From<OSLocationData> for common::Location {
    fn from(loc: OSLocationData) -> Self {
        let some_if_geq_zero = |val| (val >= 0.0).then_some(val);
        // Altitude data is only valid if the vertical accuracy is greater than
        // zero, according to the Apple CLLocation documentation.
        let altitude_valid = loc.vertical_accuracy > 0.0;
        let msl_altitude = altitude_valid.then_some(loc.msl_altitude);
        let ellipsoid_altitude =
            altitude_valid.then_some(loc.ellipsoid_altitude);
        let vertical_accuracy = altitude_valid.then_some(loc.vertical_accuracy);
        // TODO? 2023-11-15: update database to invalidate any altitudes where
        // the vertical accuracy was <= 0 or NULL. Previously we assumed they
        // were all valid. 2023-06-26 is when altitude started being logged. A
        // few points where the altitude was wrong: one at -500m and no vertical
        // accuracy, and a few points at 0m and no vertical accuracy.
        Self {
            timestamp: time::OffsetDateTime::from_unix_timestamp(loc.timestamp)
                .unwrap(),
            latitude: loc.latitude,
            longitude: loc.longitude,
            horizontal_accuracy: loc.horizontal_accuracy,
            msl_altitude,
            ellipsoid_altitude,
            vertical_accuracy,
            story: loc.story_available.then_some(loc.story),
            speed: some_if_geq_zero(loc.speed),
            speed_accuracy: some_if_geq_zero(loc.speed_accuracy),
            course: some_if_geq_zero(loc.course),
            course_accuracy: some_if_geq_zero(loc.course_accuracy),
            is_simulated_by_software: loc
                .source_info_available
                .then_some(loc.is_simulated_by_software),
            is_produced_by_accessory: loc
                .source_info_available
                .then_some(loc.is_produced_by_accessory),
            // data comes from OS -> mark as not imported
            was_imported: false,
        }
    }
}

/// Get a handle for the main database pool.
///
/// The main database is the one where new location data and pins are saved.
/// Other external databases can be mounted for viewing other data.
pub fn get_main_db_pool() -> Result<SqlitePool> {
    get_db_for_id(MAIN_DB_MOUNT_ID)
        .ok_or(anyhow::anyhow!("main database not open"))
}

/// Get a database connection for the given mount ID. ID 0 corresponds to the
/// main database, and greater ids correspond to the mounted databases.
pub fn get_db_for_id(id: MountID) -> Option<SqlitePool> {
    AppState::global().dbs.lock().unwrap().get(&id).cloned()
}

/// Checkpoint the database so all transactions in the WAL file are flushed to
/// the main database file. Also vacuum the database to reclaim unused pages and
/// save space. Vacuuming happens first since it behaves like a normal
/// transaction, then checkpointing puts all outstanding transactions into the
/// main database file.
///
/// This should be done anytime the user wants to export the database file, or
/// during app startup since migrations can cause large amounts of space to be
/// unused if they involve copying data to a new table.
pub async fn checkpoint_db(conn: &SqlitePool) {
    sqlx::query("VACUUM;")
        .execute(conn)
        .await
        .context("vacuuming db")
        .log_error_and_continue();
    // a concurrent reader (a location insert, a map query) can hold the
    // checkpoint off; the WAL drains through autocheckpoints then
    if let Err(e) = checkpoint_wal(conn).await {
        tracing::warn!("checkpointing db: {e}");
    }
}

/// Move every WAL frame into the database file and truncate the WAL, so
/// the database file alone holds all committed data. Errors if a reader
/// or writer held the checkpoint off past the busy timeout. Needs no
/// particular schema, so it runs before migrations and on export.
pub async fn checkpoint_wal(conn: &SqlitePool) -> Result<()> {
    let (busy, _log, _checkpointed): (i64, i64, i64) =
        sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE);")
            .fetch_one(conn)
            .await?;
    if busy != 0 {
        anyhow::bail!("checkpoint blocked by another connection");
    }
    Ok(())
}

/// Log a location event in the database.
pub async fn log_location_with_db<'e, E>(
    loc: impl Into<common::Location>,
    conn: E,
) -> Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    // Convert to a common::Location, which has the correct fields
    let parsed: common::Location = loc.into();
    let timestamp = parsed.timestamp.unix_timestamp();
    let (gx, gy) = grid::grid_coords(&parsed.lnglat());
    let (gx, gy) = (gx as i64, gy as i64);
    sqlx::query!(
        "INSERT INTO location (
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory,
            gx, gy
        )
        VALUES
            (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        timestamp,
        parsed.latitude,
        parsed.longitude,
        parsed.horizontal_accuracy,
        parsed.msl_altitude,
        parsed.ellipsoid_altitude,
        parsed.vertical_accuracy,
        parsed.story,
        parsed.speed,
        parsed.speed_accuracy,
        parsed.course,
        parsed.course_accuracy,
        parsed.is_simulated_by_software,
        parsed.is_produced_by_accessory,
        gx,
        gy,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Log a location in the main database. While the database is still
/// opening (init_main_db) the location is queued and inserted once it is.
pub async fn log_location(loc: OSLocationData) -> Result<()> {
    use common::state::DbStatus;
    // fast path: the pool is there
    if let Ok(conn) = get_main_db_pool() {
        return log_location_with_db(loc, &conn).await;
    }
    // The pool is re-checked under the queue's lock, since publish_main_db
    // inserts it under that lock once the queue is drained; a location
    // queued after the drain would otherwise be stranded. Without the pool,
    // the status is what says why: Opening (queue it) or Error.
    let state = AppState::global();
    let conn = {
        let mut pending = state.pending_locations.lock().unwrap();
        if let Ok(conn) = get_main_db_pool() {
            conn
        } else {
            let status = get_derived_state(|s| {
                s.mounted_dbs_on_disk.get(&MAIN_DB_MOUNT_ID).cloned()
            });
            match status {
                Some(DbStatus::Opening { .. }) => {
                    pending.push(loc);
                    return Ok(());
                }
                Some(DbStatus::Error(e)) => {
                    anyhow::bail!("main database failed to open: {e}")
                }
                // Ready is set only after the pool is inserted
                Some(DbStatus::Ready) | None => {
                    anyhow::bail!("main database not open")
                }
            }
        }
    };
    log_location_with_db(loc, &conn).await
}

/// Insert many locations into the main database in one transaction. Used for
/// bulk synthetic data generation (doc/decimation/memory-limits.md, "Probe
/// harness").
pub async fn log_locations_bulk(locs: Vec<OSLocationData>) -> Result<()> {
    let conn = get_main_db_pool()?;
    let mut tx = conn.begin().await?;
    for loc in locs {
        log_location_with_db(loc, &mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Get the last record in the database
pub async fn get_last_record() -> Option<common::Location> {
    let conn = get_main_db_pool().ok()?;
    // compile-time checked query macros are failing to infer the right type,
    // so we use the ordinary unchecked version instead for simplicity.
    match sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_all(&conn)
    .await
    {
        Ok(mut result) => result.pop().map(|l| l.into()),
        Err(e) => {
            error!("Failed to get last record: {e}");
            None
        }
    }
}

/// Count the number of locations logged in the past hour.
///
/// in SQLite, can also do time operations like so:
///     WHERE timestamp >= unixepoch('now','-1 hour')"
pub async fn count_records_past_minute() -> i32 {
    let hour_ago = time::OffsetDateTime::now_utc() - time::Duration::minutes(1);
    count_records_since(hour_ago).await
}
pub async fn count_records_past_five_minutes() -> i32 {
    let hour_ago = time::OffsetDateTime::now_utc() - time::Duration::minutes(5);
    count_records_since(hour_ago).await
}

pub async fn count_records_since(thresh: time::OffsetDateTime) -> i32 {
    let Ok(conn) = get_main_db_pool() else {
        return 0;
    };
    let timestamp = thresh.unix_timestamp();
    match sqlx::query!(
        "SELECT
            count(*) as count
        FROM location
        WHERE timestamp >= (?)",
        timestamp
    )
    .fetch_one(&conn)
    .await
    {
        Ok(result) => result.count as i32,
        Err(e) => {
            error!("Failed to count records since threshold: {e}.");
            0
        }
    }
}

async fn count_all_records(conn: &SqlitePool) -> i32 {
    match sqlx::query!(
        "SELECT
            count(*) as count
        FROM location",
    )
    .fetch_one(conn)
    .await
    {
        Ok(result) => result.count as i32,
        Err(e) => {
            error!("Failed to count all records: {e}");
            0
        }
    }
}

/// Get the range of times that encompasses all the data.
pub async fn get_time_span(conn: &SqlitePool) -> Result<Option<TimeRange>> {
    let (opt_start, opt_end) = sqlx::query_as::<_, (Option<i64>, Option<i64>)>(
        "SELECT MIN(timestamp), MAX(timestamp) FROM location",
    )
    .fetch_one(conn)
    .await?;
    let Some((start, end)) = opt_start.zip(opt_end) else {
        return Ok(None);
    };
    Ok(Some(TimeRange {
        start: Timestamp::from_second(start)?,
        end: Timestamp::from_second(end)?,
    }))
}

/// Get the timestamp of the first record in a database. Used to reset the
/// automap last updated time so imported data can be processed.
async fn get_first_timestamp(conn: &SqlitePool) -> time::OffsetDateTime {
    match sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_one(conn)
    .await
    {
        Ok(result) => common::Location::from(result).timestamp,
        Err(e) => {
            error!("Failed to get first record: {e}");
            LastAutomapUpdate::default().0
        }
    }
}

/// Import records from a database. The database must be writable so we can
/// migrate it to the current schema, if it's out of date.
pub async fn import_database_records(import_db_path: PathBuf) {
    let import_db_url = import_db_path.display().to_string();
    info!("importing file at {import_db_url}");
    // Open a connection to the database if possible
    let import_conn = match SqlitePool::connect(&import_db_url).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to open connection to import db: {e}");
            send_error_popup("Failed to open import database.");
            return;
        }
    };
    // Migrate the databse, mark all rows with was_imported=true, and close it.
    if let Err(e) = MIGRATOR.run(&import_conn).await {
        error!("Failed to migrate import db: {e}.");
        import_conn.close().await;
        send_error_popup("Failed to migrate import database.");
        return;
    }
    if let Err(e) = sqlx::query!("UPDATE location SET was_imported = 1")
        .execute(&import_conn)
        .await
    {
        error!("Failed to mark records as imported: {e}");
        import_conn.close().await;
        send_error_popup("Failed to modify import database.");
        return;
    }
    let n_to_import = count_all_records(&import_conn).await;
    let first_import_timestamp = get_first_timestamp(&import_conn).await;
    import_conn.close().await;

    let Ok(conn) = get_main_db_pool() else {
        return;
    };
    let n_initial = count_all_records(&conn).await;

    let result = sqlx::query(&format!(
        "ATTACH '{}' as toMerge;
        BEGIN;

        INSERT OR IGNORE INTO location (
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory,
            was_imported
        )
        SELECT
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory,
            was_imported
        FROM toMerge.location
        ORDER BY timestamp;

        INSERT INTO pins (
            lng, lat, name, icon, lists, tags, boundary
        )
        SELECT 
            lng, lat, name, icon, lists, tags, boundary
        FROM toMerge.pins;

        COMMIT;
        DETACH toMerge;",
        import_db_path.display()
    ))
    .execute(&conn)
    .await;
    if let Err(e) = result {
        error!("Failed to import records: {e}");
        send_error_popup("Failed to import records from database.");
        return;
    }

    let n_final = count_all_records(&conn).await;
    let n_imported = n_final - n_initial;
    // imported rows arrive without grid coords
    if let Err(e) = grid::backfill(&conn, |_, _| {}).await {
        error!("Failed to backfill grid coords after import: {e:?}");
    }

    reset_last_automap_update(&first_import_timestamp);

    update_derived_pins().await;

    let success_msg = format!(
        "Successfully imported {n_imported} records. ({} duplicates ignored.)",
        n_to_import - n_imported
    );
    info!("{}", success_msg);
    send_success_popup(&success_msg);
}

/// reset the automap latest update time (if necessary), so it can regenerate
/// for the newly imported records
fn reset_last_automap_update(first_import_timestamp: &time::OffsetDateTime) {
    let app_state = AppState::global();
    let last_automap_update = &mut app_state
        .persistent
        .lock()
        .unwrap()
        .back
        .last_automap_update
        .0;
    if *last_automap_update > *first_import_timestamp {
        *last_automap_update = *first_import_timestamp
    }
}

/// Delete the locations that are selected in the UI.
///
/// Locations are identified by timestamp, which is unique.
pub fn delete_selected_locations() {
    get_runtime().spawn(async move {
        let Some(selected_points) =
            get_front_state(|s| s.selected_points.clone())
        else {
            return;
        };
        let n_to_delete = selected_points.len();
        let mut n_deleted = 0;
        for ((mount_id, ts), _) in selected_points.iter() {
            let Some(conn) = get_db_for_id(*mount_id) else {
                error!(
                    "could not get connection for database {}",
                    db_debug_name(mount_id)
                );
                continue;
            };
            match sqlx::query("DELETE FROM location WHERE timestamp == ?")
                .bind(ts.unix_timestamp())
                .execute(&conn)
                .await
                .with_context(|| {
                    format!(
                        "deleting record with timestamp {ts:?} in database {}",
                        db_debug_name(mount_id)
                    )
                }) {
                Ok(_) => n_deleted += 1,
                Err(e) => error!("{e:?}"),
            }
        }
        send_delete_result_popup(n_to_delete, n_deleted);
        update_map_data(None, true).await;
    });
}

fn send_delete_result_popup(n_to_delete: usize, n_deleted: usize) {
    let code = PopUpCode::DeletePoints;
    let popup = if n_deleted == n_to_delete {
        let s = if n_deleted == 1 { "" } else { "s" };
        PopUp {
            kind: PopUpKind::Success,
            msg: format!("Deleted {n_deleted} point{s}"),
            code,
        }
    } else if n_deleted > 0 {
        PopUp {
            kind: PopUpKind::Error,
            msg: format!(
                "Failed to delete some points. Deleted \
                {n_deleted} of {n_to_delete}.",
            ),
            code,
        }
    } else {
        PopUp {
            kind: PopUpKind::Error,
            msg: "Failed to delete points.".into(),
            code,
        }
    };
    send_message_to_front(ToFront::PopUp(popup));
}

/// Delete the locations that are selected in the UI.
///
/// Locations are identified by timestamp, which is unique.
pub fn copy_selected_locations() {
    get_runtime().spawn(async move {
        let Some((mut selected_points, dest_db)) =
            get_front_state(|s| (s.selected_points.clone(), s.copy_dest_db))
        else {
            return;
        };
        let n_selected = selected_points.len();
        // remove points that are already in the destination database
        selected_points.retain(|((id, _), _)| *id != dest_db);
        let n_to_copy = selected_points.len();

        let Some(dest_conn) = get_db_for_id(dest_db) else {
            send_delete_result_popup(n_to_copy, 0);
            return;
        };

        let mut n_copied = 0;
        for ((mount_id, ts), _) in selected_points.iter() {
            let Some(conn) = get_db_for_id(*mount_id) else {
                error!(
                    "could not get connection for database {}",
                    db_debug_name(mount_id)
                );
                continue;
            };
            let loc = match sqlx::query_as::<_, LocationRow>(
                "SELECT * FROM location WHERE timestamp == ?",
            )
            .bind(ts.unix_timestamp())
            .fetch_one(&conn)
            .await
            .with_context(|| {
                format!(
                    "fetching record with timestamp {ts:?} in database {}",
                    db_debug_name(mount_id)
                )
            }) {
                Ok(loc) => loc,
                Err(e) => {
                    error!("{e:?}");
                    continue;
                }
            };
            match log_location_with_db(loc, &dest_conn).await.with_context(
                || {
                    format!(
                        "inserting record with timestamp {ts:?} in database {}",
                        db_debug_name(&dest_db)
                    )
                },
            ) {
                Ok(_) => n_copied += 1,
                Err(e) => error!("{e:?}"),
            }
        }
        send_copy_result_popup(n_to_copy, n_copied, n_selected);
        update_map_data(None, true).await;
    });
}

fn send_copy_result_popup(
    n_to_copy: usize,
    n_copied: usize,
    n_selected: usize,
) {
    let n_ignored = n_selected - n_to_copy;
    let n_ignored_msg = if n_ignored > 0 {
        let s = if n_ignored == 1 { "" } else { "s" };
        format!(" Ignored {n_ignored} point{s} already in database.")
    } else {
        "".into()
    };
    let code = PopUpCode::CopyPoints;
    let popup = if n_copied == n_to_copy {
        let s = if n_copied == 1 { "" } else { "s" };
        PopUp {
            kind: PopUpKind::Success,
            msg: format!("Copied {n_copied} point{s}.{n_ignored_msg}"),
            code,
        }
    } else if n_copied > 0 {
        PopUp {
            kind: PopUpKind::Error,
            msg: format!(
                "Failed to copy some points. Copied \
                {n_copied} of {n_to_copy}.{n_ignored_msg}",
            ),
            code,
        }
    } else {
        let s = if n_to_copy == 1 { "" } else { "s" };
        PopUp {
            kind: PopUpKind::Error,
            msg: format!("Failed to copy {n_to_copy} point{s}.{n_ignored_msg}"),
            code,
        }
    };
    send_message_to_front(ToFront::PopUp(popup));
}

/// Convert between the Location we have for talking to the database and the
/// Location we pass between the frontend and the backend.
///
/// We don't use the same type for each because we want the database version
/// to implement FromRow, and for common::Location to not implement it.
impl std::convert::From<LocationRow> for common::Location {
    fn from(loc: LocationRow) -> Self {
        common::Location {
            timestamp: time::OffsetDateTime::from_unix_timestamp(loc.timestamp)
                .unwrap(),
            latitude: loc.latitude,
            longitude: loc.longitude,
            horizontal_accuracy: loc.horizontal_accuracy,
            msl_altitude: loc.msl_altitude,
            ellipsoid_altitude: loc.ellipsoid_altitude,
            vertical_accuracy: loc.vertical_accuracy,
            story: loc.story,
            speed: loc.speed,
            speed_accuracy: loc.speed_accuracy,
            course: loc.course,
            course_accuracy: loc.course_accuracy,
            is_simulated_by_software: loc.is_simulated_by_software,
            is_produced_by_accessory: loc.is_produced_by_accessory,
            was_imported: loc.was_imported,
        }
    }
}

/// A spatial decimation cell: grid cells gx/gy >> shift (database/grid.rs).
///
/// Spatial decimation keeps only the most recent point in each cell, which is
/// lossless for point rendering when the cell is about the size of a pixel.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SpatialCell {
    pub grid_shift: u32,
}

impl SpatialCell {
    /// Cell that is at least `pixels` wide on a Web Mercator map at the
    /// given zoom.
    pub fn from_zoom(zoom: f64, pixels: f64) -> Self {
        Self {
            grid_shift: grid::query_shift(zoom, pixels),
        }
    }
}

/// Inclusive rectangle in grid units (database/grid.rs).
struct GridRect {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

/// Spatial mode: rows in the time range under which the query filters by
/// time first (timestamp index, then cells on the rows in range) rather
/// than walking the grid. The time path costs O(rows in range); the walk
/// costs O(occupied cells in view) whatever the time range, and occupied
/// cells have no cheap estimate. The crossover is 150k-800k rows
/// depending on zoom (doc/decimation/probe-results/time-range-bench.md);
/// one constant is within 2x of optimal everywhere, and both paths cost
/// ~100 ms there.
pub const TIME_PATH_MAX_ROWS: u64 = 300_000;

/// Temporal mode: cap on the route estimates (rows in range, points in
/// view). Index-only counts cost ~15-30 ms per million entries, so this
/// can be higher than TIME_PATH_MAX_ROWS; past it the routes are within
/// ~2x of each other.
pub const TEMPORAL_COUNT_CAP: u64 = 1_000_000;

/// Whether the planner should use the timestamp index for the time range.
/// Grid queries pin their index with INDEXED BY, but the walk's subqueries
/// hint against it (likelihood 1.0) so a wide range doesn't pull them onto
/// a scan of the range; the time route wants exactly that index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeHint {
    AvoidIndex,
    UseIndex,
}

/// Which index serves the map query. See fetch_decimated_result_with_db.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// Timestamp index: rows in the time range, then bounds or cells.
    Time,
    /// Grid level indexes (database/grid.rs).
    Grid,
}

/// Records returned by a decimated fetch, with how they were decimated.
pub struct DecimatedResult {
    pub records: Vec<NarrowPoint>,
    /// True if the records are a thinned sample of the matching points:
    /// every nth point in temporal mode, or bucketed in spatial mode.
    pub decimated: bool,
    /// True if the records were spatially bucketed, in which case they are not
    /// a time-uniform sample and consecutive points should not be joined with
    /// lines.
    pub spatially_bucketed: bool,
    /// True if the hard_cap memory backstop bound the result: a coarser
    /// sample than the threshold alone would give in temporal mode, the
    /// oldest cells dropped in spatial mode.
    pub memory_capped: bool,
}

#[derive(Default, bon::Builder)]
pub struct FilteredQuery {
    start: Option<jiff::Timestamp>,
    end: Option<jiff::Timestamp>,
    filters: Option<Vec<Filter>>,
    /// The lnglat bounds for the query.
    bounds: Option<LngLatBounds>,
    /// Whether to get the next/previous point just outside the lnglat bounds
    /// (only works for decimation queries). Does not affect the decimation.
    #[builder(default)]
    get_adjacent: bool,
    /// Number of points returned by first_n queries.
    limit: Option<u64>,
    /// Point count above which decimation queries thin the result. In
    /// temporal mode it is also the target: the decimation factor is chosen
    /// so the result lands under it. In spatial mode it is only the trigger;
    /// the result size is the number of occupied cells.
    decimation_threshold: Option<u64>,
    /// If set, decimation queries over the threshold keep the most recent
    /// point per cell instead of every nth point. get_adjacent is ignored
    /// when bucketing.
    spatial_cell: Option<SpatialCell>,
    /// Database column fetched into NarrowPoint::field1 by the decimated
    /// fetches, CAST to REAL. The caller keeps track of what it means.
    field1: Option<&'static str>,
    /// Same for NarrowPoint::field2.
    field2: Option<&'static str>,
    /// Never-bind memory backstop (doc/decimation/memory-limits.md,
    /// "Backend memory backstop"). Clamps the decimation threshold, and bounds
    /// bucketed fetches to the most recent hard_cap cells (re-sorted
    /// ascending), since the threshold doesn't bound the result there.
    /// DecimatedResult::memory_capped reports when it binds.
    hard_cap: Option<u64>,
}

use filtered_query_builder::{IsUnset, SetEnd, SetStart, State};

impl<S: State> FilteredQueryBuilder<S> {
    /// Custom builder method for setting the start and end from a time range.
    pub fn time_range(
        self,
        time_range: TimeRange,
    ) -> FilteredQueryBuilder<SetEnd<SetStart<S>>>
    where
        S::Start: IsUnset,
        S::End: IsUnset,
    {
        self.start(time_range.start).end(time_range.end)
    }
}

impl<'a> FilteredQuery {
    /// Select list of the narrow decimated fetches: the fixed NarrowPoint
    /// columns plus the caller's field1/field2. The CAST makes INTEGER
    /// columns decode as f64 (sqlx type-checks against the value's
    /// storage class, not the declared affinity) and is a no-op on REAL
    /// ones. `p` prefixes every column with a table alias where needed.
    fn select_list(&self, p: &str) -> String {
        let field = |col: Option<&str>, name: &str| match col {
            Some(c) => format!("CAST({p}{c} AS REAL) AS {name}"),
            None => format!("NULL AS {name}"),
        };
        format!(
            "{p}id, {p}timestamp, {p}latitude, {p}longitude, {}, {}",
            field(self.field1, "field1"),
            field(self.field2, "field2"),
        )
    }

    /// Create a query that counts the number of records inside the view bounds
    fn count_query(
        &self,
        explain_query_plan: bool,
        decim: i64,
        hint: TimeHint,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push("SELECT count(*) FROM location WHERE 1");
        add_bounds_to_query(&self.bounds, &mut q);
        add_decim_to_query(decim, &mut q);
        self.add_basic_filters_hint(&mut q, hint);
        q
    }

    /// Create a query that fetches records that are bounded and spaced
    /// according to the decimation factor.
    fn bounded_decim_query(
        &self,
        explain_query_plan: bool,
        decim: i64,
        hint: TimeHint,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push(format!(
            "SELECT {} FROM location WHERE 1",
            self.select_list("")
        ));
        add_bounds_to_query(&self.bounds, &mut q);
        add_decim_to_query(decim, &mut q);
        self.add_basic_filters_hint(&mut q, hint);
        q
    }

    /// Create a query that fetches the most recent record in each spatial cell
    /// inside the view bounds. The bare columns of a SQLite aggregate query
    /// (including expressions over them, like the select list's CASTs) come
    /// from the row holding max(timestamp), so this yields that row's values.
    fn bucketed_query(
        &self,
        explain_query_plan: bool,
        shift: u32,
        hint: TimeHint,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push(format!(
            "SELECT {}, max(timestamp) FROM location WHERE 1",
            self.select_list("")
        ));
        add_bounds_to_query(&self.bounds, &mut q);
        self.add_basic_filters_hint(&mut q, hint);
        q.push(format!(" GROUP BY gy >> {shift}, gx >> {shift}"));
        if let Some(cap) = self.hard_cap {
            // Most recent cells first so the cap keeps recent data.
            // Re-sorted ascending in Rust.
            q.push(" ORDER BY max(timestamp) DESC LIMIT ");
            q.push_bind(cap as i64);
        }
        q
    }

    /// Create a query that fetches a data point before the supplied timestamp
    fn adjacent_before_query(
        &self,
        explain_query_plan: bool,
        decim: i64,
        timestamp: i64,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push(format!(
            "SELECT {} FROM location WHERE 1",
            self.select_list("")
        ));
        self.add_basic_filters(&mut q);
        add_decim_to_query(decim, &mut q);
        q.push(" AND timestamp < ");
        q.push_bind(timestamp);
        q.push(" ORDER BY timestamp DESC LIMIT 1");
        q
    }

    /// Create a query that fetches a data point after the supplied timestamp
    fn adjacent_after_query(
        &self,
        explain_query_plan: bool,
        decim: i64,
        timestamp: i64,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push(format!(
            "SELECT {} FROM location WHERE 1",
            self.select_list("")
        ));
        self.add_basic_filters(&mut q);
        add_decim_to_query(decim, &mut q);
        q.push(" AND timestamp > ");
        q.push_bind(timestamp);
        q.push(" ORDER BY timestamp ASC LIMIT 1");
        q
    }

    /// Add the simpler filters to the query that don't require special
    /// consideration. (Time and user-defined filters)
    fn add_basic_filters(&self, q: &mut QueryBuilder<Sqlite>) {
        self.add_basic_filters_hint(q, TimeHint::AvoidIndex);
    }

    fn add_basic_filters_hint(
        &self,
        q: &mut QueryBuilder<Sqlite>,
        hint: TimeHint,
    ) {
        add_start_time_to_query(&self.start, q, hint);
        add_end_time_to_query(&self.end, q, hint);
        add_filters_to_query(&self.filters, q);
    }

    /// Rows in the time range, counted on the timestamp index and stopping
    /// at `limit + 1`. None when the range is open on both ends.
    async fn time_range_rows(
        &self,
        conn: &SqlitePool,
        limit: u64,
    ) -> Option<u64> {
        if self.start.is_none() && self.end.is_none() {
            return None;
        }
        let mut q = new_query(false);
        q.push("SELECT count(*) FROM (SELECT 1 FROM location WHERE 1");
        add_start_time_to_query(&self.start, &mut q, TimeHint::UseIndex);
        add_end_time_to_query(&self.end, &mut q, TimeHint::UseIndex);
        q.push(" LIMIT ");
        q.push_bind(limit as i64 + 1);
        q.push(")");
        q.build_query_scalar::<i64>()
            .fetch_one(conn)
            .await
            .ok()
            .map(|n| n as u64)
    }

    /// Route input: the rows in the time range up to the mode's cap (None
    /// with no range).
    async fn route_inputs(&self, conn: &SqlitePool) -> Option<u64> {
        let cap = match self.spatial_cell {
            Some(_) => TIME_PATH_MAX_ROWS,
            None => TEMPORAL_COUNT_CAP,
        };
        self.time_range_rows(conn, cap).await
    }

    /// Count the number of points in the visible region, and use that to
    /// calculate the decimation factor for the query. The count is sped up by
    /// sampling every 10th point, then multiplying this count by 10. (Speeds up
    /// 330 ms -> 120 ms.) Returns (decimation factor, memory_capped): the
    /// threshold is clamped to hard_cap, and memory_capped is set when that
    /// clamp coarsens the sample beyond what the threshold alone would give.
    async fn fetch_decim(
        &self,
        conn: &SqlitePool,
        hint: TimeHint,
    ) -> (i64, bool) {
        let factor = 10;
        let mut q = self.count_query(false, factor, hint);
        let query_as = q.build_query_as::<CountResult>();

        let reduced_count = query_as.fetch_one(conn).await.unwrap().0;

        // Since we decimate this count, we need to make sure the it's not 0
        self.decim_from_count(reduced_count.max(1) * factor)
    }

    /// Decimation factor for a count of matching points, and whether the
    /// hard_cap clamp coarsened it beyond what the threshold alone would
    /// give.
    fn decim_from_count(&self, count: i64) -> (i64, bool) {
        let count = count.max(1);
        // (count - 1)/n + 1 = ceil without overflowing huge thresholds
        let decim_for = |n: i64| (count - 1) / n + 1;
        let thresh = self.threshold() as i64;
        match self.hard_cap.map(|c| (c as i64).max(1)) {
            Some(cap) if cap < thresh => {
                let decim = decim_for(cap);
                (decim, decim > decim_for(thresh))
            }
            _ => (decim_for(thresh), false),
        }
    }

    fn threshold(&self) -> u64 {
        self.decimation_threshold.unwrap_or(10000)
    }

    /// Point count above which spatial mode buckets: min(threshold,
    /// hard_cap).
    fn bucketing_trigger(&self) -> i64 {
        self.threshold().min(self.hard_cap.unwrap_or(u64::MAX)) as i64
    }

    /// Whether more than the bucketing trigger points match: a count that
    /// stops scanning once it passes that many rows.
    async fn fetch_over_trigger(
        &self,
        conn: &SqlitePool,
        hint: TimeHint,
    ) -> bool {
        let n = self.bucketing_trigger();
        let mut q = new_query(false);
        q.push("SELECT count(*) FROM (SELECT 1 FROM location WHERE 1");
        add_bounds_to_query(&self.bounds, &mut q);
        self.add_basic_filters_hint(&mut q, hint);
        q.push(" LIMIT ");
        q.push_bind(n + 1);
        q.push(")");
        // On a query error, bucket: the memory backstop must fail toward
        // fewer points, not an unbounded fetch
        let count: i64 = q
            .build_query_scalar()
            .fetch_one(conn)
            .await
            .unwrap_or_else(|e| {
                error!("trigger count query failed, assuming over: {e}");
                n + 1
            });
        count > n
    }

    /// Fetch the points in the bounded region
    async fn fetch_bounded(
        &self,
        conn: &SqlitePool,
        decim: i64,
        hint: TimeHint,
    ) -> Vec<NarrowPoint> {
        let mut q = self.bounded_decim_query(false, decim, hint);
        let query_as = q.build_query_as::<NarrowPoint>();
        let mut bounded_recs = query_as.fetch_all(conn).await.unwrap();

        // Use Rust's timsort-like alg to quickly sort the mostly sorted result
        bounded_recs.sort_by_key(|r| r.timestamp);

        bounded_recs
    }

    /// 2-D bounds filter over `rect` on a grid level index, for the
    /// temporal-mode count and fetch. A recursive CTE over the cell rows of
    /// the rectangle joins the index one row at a time (one seek per row
    /// rather than a scan of the whole cy band). Cells strictly inside the
    /// rectangle are accepted from the index alone; edge cells read the
    /// row for the exact bounds, so the result is exact at any zoom. The
    /// level comes from grid::filter_level. `select` is the select list.
    fn grid_filter_query(
        &self,
        rect: &GridRect,
        select: &str,
        prefix: &str,
    ) -> QueryBuilder<'a, Sqlite> {
        let level = grid::filter_level(rect.x1 - rect.x0, rect.y1 - rect.y0);
        let s = grid::level_shift(level);
        let idx = grid::index_name(level);
        let (cx0, cx1) = ((rect.x0 >> s) as i64, (rect.x1 >> s) as i64);
        let (cy0, cy1) = ((rect.y0 >> s) as i64, (rect.y1 >> s) as i64);
        // the interior of the rectangle, in cells
        let (icx0, icx1) = (cx0 + 1, cx1 - 1);
        let (icy0, icy1) = (cy0 + 1, cy1 - 1);
        let (x0, x1, y0, y1) = (rect.x0, rect.x1, rect.y0, rect.y1);
        let mut q = new_query(false);
        q.push(prefix);
        q.push(format!(
            r#"
WITH RECURSIVE r(cy) AS (
  SELECT {cy0}
  UNION ALL
  SELECT cy + 1 FROM r WHERE cy < {cy1}
)
SELECT {select}
FROM r
JOIN location INDEXED BY {idx}
  ON cy{level} = r.cy AND cx{level} BETWEEN {cx0} AND {cx1}
WHERE (
  -- cells strictly inside the rectangle pass on the index alone
  (cy{level} BETWEEN {icy0} AND {icy1} AND
   cx{level} BETWEEN {icx0} AND {icx1})
  -- edge cells check the row's exact grid coordinates
  OR (gy BETWEEN {y0} AND {y1} AND gx BETWEEN {x0} AND {x1})
)"#
        ));
        q
    }

    /// Exact count of matching points via the grid (no sampling: the count
    /// is index-only apart from edge cells). With `limit`, counting stops
    /// past it (per rectangle), for a cheap "fewer than" test; with
    /// `filters` false the time range and datastream filters are left out,
    /// keeping the count index-only (the cost estimate of the grid route,
    /// which scans the view's index entries whatever the filters). None on
    /// a query error.
    async fn fetch_count_grid(
        &self,
        conn: &SqlitePool,
        limit: Option<i64>,
        filters: bool,
    ) -> Option<i64> {
        let mut total = 0;
        for rect in self.grid_rects() {
            let mut q = match limit {
                Some(_) => {
                    self.grid_filter_query(&rect, "1", "SELECT count(*) FROM (")
                }
                None => self.grid_filter_query(&rect, "count(*)", ""),
            };
            if filters {
                self.add_basic_filters(&mut q);
            }
            if let Some(limit) = limit {
                q.push(" LIMIT ");
                q.push_bind(limit + 1);
                q.push(")");
            }
            match q.build_query_scalar::<i64>().fetch_one(conn).await {
                Ok(n) => total += n,
                Err(e) => {
                    error!("grid count query failed, falling back: {e}");
                    return None;
                }
            }
        }
        Some(total)
    }

    /// Grid version of fetch_bounded. None on a query error.
    async fn fetch_bounded_grid(
        &self,
        conn: &SqlitePool,
        decim: i64,
    ) -> Option<Vec<NarrowPoint>> {
        let mut recs = Vec::new();
        for rect in self.grid_rects() {
            let select = self.select_list("location.");
            let mut q = self.grid_filter_query(&rect, &select, "");
            add_decim_to_query(decim, &mut q);
            self.add_basic_filters(&mut q);
            match q.build_query_as::<NarrowPoint>().fetch_all(conn).await {
                Ok(rows) => recs.extend(rows),
                Err(e) => {
                    error!("grid bounded query failed, falling back: {e}");
                    return None;
                }
            }
        }
        recs.sort_by_key(|r| r.timestamp);
        Some(recs)
    }

    /// Fetch the most recent point in each spatial cell, sorted by timestamp.
    /// The bool is true if hard_cap truncated the result to the most recent
    /// cells.
    async fn fetch_bucketed(
        &self,
        conn: &SqlitePool,
        shift: u32,
        hint: TimeHint,
    ) -> (Vec<NarrowPoint>, bool) {
        let mut q = self.bucketed_query(false, shift, hint);
        let query_as = q.build_query_as::<NarrowPoint>();
        let mut recs = query_as.fetch_all(conn).await.unwrap();
        // Hitting the cap reads as truncation; the false positive when the
        // cell count lands exactly on the cap is harmless (it's one row
        // from true anyway).
        let memory_capped =
            self.hard_cap.is_some_and(|cap| recs.len() as u64 >= cap);
        recs.sort_by_key(|r| r.timestamp);
        (recs, memory_capped)
    }

    /// Grid version of fetch_bucketed: the newest point per grid query cell
    /// in view, via the level indexes (database/grid.rs). Bounds beyond
    /// +-180 become a second x range. The hard_cap keeps the most recent
    /// cells, as in fetch_bucketed. None on a query error, so the caller
    /// can fall back to the time route instead of failing the map update.
    async fn fetch_bucketed_grid(
        &self,
        conn: &SqlitePool,
        shift: u32,
    ) -> Option<(Vec<NarrowPoint>, bool)> {
        let mut recs = Vec::new();
        for rect in self.grid_rects() {
            let mut q = self.grid_bucketed_query(&rect, shift);
            let query_as = q.build_query_as::<NarrowPoint>();
            match query_as.fetch_all(conn).await {
                Ok(rows) => recs.extend(rows),
                Err(e) => {
                    error!("grid bucketed query failed, falling back: {e}");
                    return None;
                }
            }
        }
        // most recent cells first, so the cap keeps recent data
        recs.sort_by_key(|r| std::cmp::Reverse(r.timestamp));
        let memory_capped =
            self.hard_cap.is_some_and(|cap| recs.len() as u64 >= cap);
        if let Some(cap) = self.hard_cap {
            recs.truncate(cap as usize);
        }
        recs.sort_by_key(|r| r.timestamp);
        Some((recs, memory_capped))
    }

    /// Grid rectangles (inclusive, in grid units) covering the view bounds.
    /// One rectangle normally; two when the view spills past the
    /// antimeridian onto an aliased copy of the world; the whole world when
    /// unbounded or spilling both ways.
    fn grid_rects(&self) -> Vec<GridRect> {
        let world = GridRect {
            x0: 0,
            x1: u32::MAX,
            y0: 0,
            y1: u32::MAX,
        };
        let Some(b) = &self.bounds else {
            return vec![world];
        };
        let alias_positive = b.sw.lng < -180.;
        let alias_negative = b.ne.lng > 180.;
        if alias_positive && alias_negative {
            return vec![world];
        }
        // y grows southward
        let (_, y0) = grid::grid_coords(&LngLat {
            lng: 0.,
            lat: b.ne.lat,
        });
        let (_, y1) = grid::grid_coords(&LngLat {
            lng: 0.,
            lat: b.sw.lat,
        });
        let x_of = |lng: f64| {
            grid::grid_coords(&LngLat {
                lng: lng.clamp(-180., 180.),
                lat: 0.,
            })
            .0
        };
        // clamp(180) maps to x = 0 after wrapping, so use the max for it
        let x_hi = |lng: f64| if lng >= 180. { u32::MAX } else { x_of(lng) };
        let mut rects = vec![GridRect {
            x0: x_of(b.sw.lng),
            x1: x_hi(b.ne.lng),
            y0,
            y1,
        }];
        if alias_positive {
            rects.push(GridRect {
                x0: x_of(b.sw.lng + 360.),
                x1: u32::MAX,
                y0,
                y1,
            });
        }
        if alias_negative {
            rects.push(GridRect {
                x0: 0,
                x1: x_hi(b.ne.lng - 360.),
                y0,
                y1,
            });
        }
        rects
    }

    /// The newest row per query cell (grid units >> `shift`) inside `rect`,
    /// as narrow rows (select_list).
    ///
    /// Walk form (index cell not coarser than the query cell): a recursive CTE
    /// seeks from occupied index cell to occupied index cell inside the
    /// rectangle (a loose index scan; SQLite won't plan one itself), takes
    /// each cell's newest passing timestamp with one more seek, groups
    /// those by query cell, and joins the winners' rows back on the UNIQUE
    /// timestamp. Cost is O(occupied index cells) instead of O(points in
    /// view). Positions are packed (cy << 32 | cx) so one scalar subquery
    /// per step carries both (safe: cell coordinates are at most 22 bits);
    /// "real" marks positions that are occupied cells inside the x range
    /// rather than jump targets.
    ///
    /// Filter form (query cell finer than every index): the coarsest index
    /// is the 2-D bounds filter and the grouping reads gx/gy from rows.
    fn grid_bucketed_query(
        &self,
        rect: &GridRect,
        shift: u32,
    ) -> QueryBuilder<'a, Sqlite> {
        const M: u64 = 0xffff_ffff;
        let mut q = new_query(false);
        match grid::index_for(shift) {
            grid::IndexUse::Walk { level } => {
                let idx = grid::index_name(level);
                let s = grid::level_shift(level);
                let d = shift - s;
                let (cx0, cx1) = (rect.x0 >> s, rect.x1 >> s);
                let (cy0, cy1) = (rect.y0 >> s, rect.y1 >> s);
                // packed (cy << 32 | cx) of the first occupied cell after
                // position `from`, on rows up to cy1
                let seek = |from: &str| {
                    format!(
                        r#"
  (SELECT (cy{level} << 32) + cx{level}
   FROM location INDEXED BY {idx}
   WHERE (cy{level}, cx{level}) > (({from}) >> 32, ({from}) & {M})
     AND cy{level} <= {cy1}
   ORDER BY cy{level}, cx{level} LIMIT 1)"#
                    )
                };
                // next position after the found cell w.nxt: the cell
                // itself if inside the x range, else a jump to just before
                // cx0 on the same row (cell was left of the range) or on
                // the next row (right of it)
                let pos = format!(
                    r#"
  (CASE WHEN (w.nxt & {M}) < {cx0}
          THEN (w.nxt >> 32 << 32) + {cx0} - 1
        WHEN (w.nxt & {M}) > {cx1}
          THEN (((w.nxt >> 32) + 1) << 32) + {cx0} - 1
        ELSE w.nxt END)"#
                );
                let p0 = format!("(({cy0} << 32) + {cx0} - 1)");
                q.push(format!(
                    r#"
WITH RECURSIVE w(pos, real, nxt) AS (
  -- seed: just before the rectangle's first cell (real = 0)
  SELECT {p0}, 0, {seek_first}
  UNION ALL
  SELECT {pos},
  -- real: nxt is an occupied cell inside the x range, not a jump
  (w.nxt & {M}) BETWEEN {cx0} AND {cx1}, {seek_next}
  FROM w WHERE w.nxt IS NOT NULL
)
-- newest passing timestamp of each walked cell, grouped by query cell
SELECT {select} FROM (
  SELECT max(latest) AS ts FROM (
    SELECT
      (SELECT timestamp FROM location INDEXED BY {idx}
       WHERE cy{level} = w.pos >> 32 AND cx{level} = w.pos & {M}"#,
                    select = self.select_list("l."),
                    seek_first = seek(&p0),
                    seek_next = seek(&pos),
                ));
                self.add_basic_filters(&mut q);
                q.push(format!(
                    r#"
       ORDER BY timestamp DESC LIMIT 1) AS latest,
      (w.pos >> 32) >> {d} AS qy,
      (w.pos & {M}) >> {d} AS qx
    FROM w WHERE real
  ) WHERE latest IS NOT NULL
  GROUP BY qy, qx
) g
-- timestamp is UNIQUE: one autoindex seek per winning row
JOIN location l ON l.timestamp = g.ts"#
                ));
            }
            grid::IndexUse::Filter { level } => {
                let idx = grid::index_name(level);
                let s = grid::level_shift(level);
                q.push(format!(
                    r#"
SELECT {select} FROM (
  SELECT id, max(timestamp)
  FROM location INDEXED BY {idx}
  -- the coarse index narrows to the rectangle; gx/gy are exact
  WHERE cy{level} BETWEEN {cy0} AND {cy1}
    AND cx{level} BETWEEN {cx0} AND {cx1}
    AND gy BETWEEN {y0} AND {y1}
    AND gx BETWEEN {x0} AND {x1}"#,
                    select = self.select_list("l."),
                    cy0 = rect.y0 >> s,
                    cy1 = rect.y1 >> s,
                    cx0 = rect.x0 >> s,
                    cx1 = rect.x1 >> s,
                    y0 = rect.y0,
                    y1 = rect.y1,
                    x0 = rect.x0,
                    x1 = rect.x1,
                ));
                self.add_basic_filters(&mut q);
                q.push(format!(
                    r#"
  GROUP BY gy >> {shift}, gx >> {shift}
) g
JOIN location l ON l.id = g.id"#
                ));
            }
        }
        if let Some(cap) = self.hard_cap {
            q.push(" ORDER BY l.timestamp DESC LIMIT ");
            q.push_bind(cap as i64);
        }
        q
    }

    /// Fetch the points adjacent to the bounded region.
    async fn fetch_adjacent(
        &self,
        conn: &SqlitePool,
        decim: i64,
        mut recs: Vec<NarrowPoint>,
    ) -> Vec<NarrowPoint> {
        // let before = std::time::Instant::now();
        let mut new_recs = vec![]; // (index to insert, rec)
        for (i, rec) in recs.iter().enumerate() {
            let mut fetch_before = true;
            let mut fetch_after = true;
            // check if the before and after records are offset by decim. if
            // they are, we can skip fetching adjacent points in that direction.
            if i > 0 {
                let before = &recs[i - 1];
                if before.id + decim == rec.id {
                    fetch_before = false;
                }
            }
            if i < recs.len() - 1 {
                let after = &recs[i + 1];
                if rec.id + decim == after.id {
                    fetch_after = false;
                }
            }
            let should_push_new_rec = |new_rec: &NarrowPoint, idx, before| {
                // check existing records to see if we fetched a duplicate
                if (i != 0 && before) || (i != recs.len() - 1 && !before) {
                    let already_there_idx =
                        if before { idx - 1 } else { idx + 1 };
                    if let Some(already_there) = recs.get(already_there_idx) {
                        let already_there: &NarrowPoint = already_there;
                        if already_there.id == new_rec.id {
                            return false; // duplicate, don't push
                        }
                    }
                }
                true
            };
            if fetch_before {
                let ts = rec.timestamp.unix_timestamp();
                let mut q = self.adjacent_before_query(false, decim, ts);
                let query_as = q.build_query_as::<NarrowPoint>();
                let mut maybe_new = query_as.fetch_all(conn).await.unwrap();
                if let Some(new_rec) = maybe_new.pop() {
                    if should_push_new_rec(&new_rec, i, true) {
                        new_recs.push((i, new_rec));
                    }
                }
            }
            if fetch_after {
                let ts = rec.timestamp.unix_timestamp();
                let mut q = self.adjacent_after_query(false, decim, ts);
                let query_as = q.build_query_as::<NarrowPoint>();
                let mut maybe_new = query_as.fetch_all(conn).await.unwrap();
                if let Some(new_rec) = maybe_new.pop() {
                    if should_push_new_rec(&new_rec, i, false) {
                        new_recs.push((i + 1, new_rec));
                    }
                }
            }
        }
        // println!("adjacent took: {:.6?}", before.elapsed());
        // insert the new data into the sorted array
        let mut offset = 0;
        for (insert_idx, new_rec) in new_recs.into_iter() {
            if insert_idx > 0 {
                if let Some(before) = recs.get(insert_idx + offset - 1) {
                    if before.id == new_rec.id {
                        // The backwards adjacent point is the same as the
                        // previous bounded point's forward adjacent point ->
                        // don't duplicate
                        continue;
                    }
                }
            }
            recs.insert(insert_idx + offset, new_rec);
            offset += 1;
        }
        recs
    }

    /// Fetch points while decimating to keep under the threshold. When
    /// everything fits under it, all points are returned regardless of the
    /// decimation mode.
    ///
    /// Routing (doc/decimation/probe-results/time-range-bench.md): every
    /// route walks index entries, and the cheaper route is the one with
    /// fewer. Rows in the time range (timestamp index) and, in temporal
    /// mode, points in view (grid index) are counted index-only with a
    /// LIMIT; spatial mode's walk cost has no cheap estimate, so it uses
    /// a threshold on rows in range instead.
    pub async fn fetch_decimated_result_with_db(
        &self,
        conn: &SqlitePool,
    ) -> DecimatedResult {
        let rows = self.route_inputs(conn).await;
        match self.spatial_cell {
            Some(cell) => self.fetch_spatial(conn, cell, rows).await,
            None => self.fetch_temporal(conn, rows).await,
        }
    }

    /// Temporal decimation: every nth point under the threshold. Grid when
    /// the view holds fewer points than the range holds rows; ties and
    /// both over the cap go to the time route when a range is set (a
    /// sequential scan in rowid order, which beats any index once the
    /// view holds most of the data), else the grid.
    async fn fetch_temporal(
        &self,
        conn: &SqlitePool,
        rows_in_range: Option<u64>,
    ) -> DecimatedResult {
        const CAP: i64 = TEMPORAL_COUNT_CAP as i64;
        // capped or unbounded range counts as over the cap
        let n_time = rows_in_range.map_or(CAP + 1, |n| n as i64);
        let limit = n_time.min(CAP);
        let n_view = self
            .fetch_count_grid(conn, Some(limit), false)
            .await
            .unwrap_or(limit + 1);
        let route =
            if n_view < n_time || (n_view > CAP && rows_in_range.is_none()) {
                Route::Grid
            } else {
                Route::Time
            };
        let (decim, memory_capped, mut recs) = match route {
            Route::Grid => match self.fetch_temporal_grid(conn).await {
                Some(r) => r,
                None => self.fetch_temporal_time(conn).await,
            },
            Route::Time => self.fetch_temporal_time(conn).await,
        };
        let fmt = |n: i64| {
            if n > CAP {
                format!(">{CAP}")
            } else {
                n.to_string()
            }
        };
        debug!(
            "temporal decimation {route:?}: time/view {}/{}, decim {decim}, \
             {} rows",
            fmt(n_time),
            fmt(n_view),
            recs.len()
        );
        if self.bounds.is_some() && self.get_adjacent {
            recs = self.fetch_adjacent(conn, decim, recs).await;
        }
        DecimatedResult {
            records: recs,
            decimated: decim > 1,
            spatially_bucketed: false,
            memory_capped,
        }
    }

    /// Temporal decimation through the timestamp index (or a table scan
    /// without a range, in rowid order either way).
    async fn fetch_temporal_time(
        &self,
        conn: &SqlitePool,
    ) -> (i64, bool, Vec<NarrowPoint>) {
        let (decim, memory_capped) =
            self.fetch_decim(conn, TimeHint::UseIndex).await;
        let recs = self.fetch_bounded(conn, decim, TimeHint::UseIndex).await;
        (decim, memory_capped, recs)
    }

    /// Temporal decimation through the grid: exact count, then the
    /// decimated fetch. None on a query error.
    async fn fetch_temporal_grid(
        &self,
        conn: &SqlitePool,
    ) -> Option<(i64, bool, Vec<NarrowPoint>)> {
        let count = self.fetch_count_grid(conn, None, true).await?;
        let (decim, memory_capped) = self.decim_from_count(count);
        let recs = self.fetch_bounded_grid(conn, decim).await?;
        Some((decim, memory_capped, recs))
    }

    /// fetch_bounded on the given route: the grid indexes when `route` is
    /// Grid (falling back to the time route on a query error), else the
    /// time route with `hint`.
    async fn fetch_bounded_routed(
        &self,
        conn: &SqlitePool,
        decim: i64,
        route: Route,
        hint: TimeHint,
    ) -> Vec<NarrowPoint> {
        if route == Route::Grid {
            if let Some(recs) = self.fetch_bounded_grid(conn, decim).await {
                return recs;
            }
        }
        self.fetch_bounded(conn, decim, hint).await
    }

    /// fetch_bucketed on the given route, with the same fallback as
    /// fetch_bounded_routed.
    async fn fetch_bucketed_routed(
        &self,
        conn: &SqlitePool,
        shift: u32,
        route: Route,
        hint: TimeHint,
    ) -> (Vec<NarrowPoint>, bool) {
        if route == Route::Grid {
            if let Some(r) = self.fetch_bucketed_grid(conn, shift).await {
                return r;
            }
        }
        self.fetch_bucketed(conn, shift, hint).await
    }

    /// Spatial decimation: the newest point per cell once over the trigger.
    async fn fetch_spatial(
        &self,
        conn: &SqlitePool,
        cell: SpatialCell,
        rows_in_range: Option<u64>,
    ) -> DecimatedResult {
        let route = match rows_in_range {
            Some(n) if n <= TIME_PATH_MAX_ROWS => Route::Time,
            _ => Route::Grid,
        };
        let hint = match route {
            Route::Time => TimeHint::UseIndex,
            Route::Grid => TimeHint::AvoidIndex,
        };
        // Only whether the view is over the bucketing trigger is needed,
        // which a LIMIT-bounded count answers in O(trigger) rows: on the
        // grid, else the timestamp index or a scan
        let over = if route == Route::Grid {
            let trigger = self.bucketing_trigger();
            match self.fetch_count_grid(conn, Some(trigger), true).await {
                Some(n) => n > trigger,
                None => self.fetch_over_trigger(conn, hint).await,
            }
        } else {
            self.fetch_over_trigger(conn, hint).await
        };
        if !over {
            let mut recs =
                self.fetch_bounded_routed(conn, 1, route, hint).await;
            if self.bounds.is_some() && self.get_adjacent {
                recs = self.fetch_adjacent(conn, 1, recs).await;
            }
            return DecimatedResult {
                records: recs,
                decimated: false,
                spatially_bucketed: false,
                memory_capped: false,
            };
        }
        let shift = cell.grid_shift;
        let (recs, memory_capped) =
            self.fetch_bucketed_routed(conn, shift, route, hint).await;
        debug!(
            "spatial decimation {route:?}: {} in range, {} cells",
            rows_in_range.map_or("no range".to_string(), |n| n.to_string()),
            recs.len()
        );
        DecimatedResult {
            records: recs,
            decimated: true,
            spatially_bucketed: true,
            memory_capped,
        }
    }

    /// Fetch points while decimating to keep under the threshold
    pub async fn fetch_decimated_with_db(
        &self,
        conn: &SqlitePool,
    ) -> Vec<NarrowPoint> {
        self.fetch_decimated_result_with_db(conn).await.records
    }

    pub async fn fetch_decimated(&self) -> Vec<NarrowPoint> {
        let Ok(conn) = get_main_db_pool() else {
            return Vec::new();
        };
        self.fetch_decimated_with_db(&conn).await
    }

    /// fetch_decimated, hydrated to full Locations by a second lookup on
    /// the UNIQUE timestamps. For callers that need more than the narrow
    /// columns (export); the map path never pays for the full rows.
    pub async fn fetch_decimated_full(&self) -> Vec<common::Location> {
        let Ok(conn) = get_main_db_pool() else {
            return Vec::new();
        };
        let narrow = self.fetch_decimated_with_db(&conn).await;
        fetch_full_locations(&conn, &narrow).await
    }

    fn first_n_query(
        &self,
        explain_query_plan: bool,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push("SELECT * FROM location WHERE 1");
        add_bounds_to_query(&self.bounds, &mut q);
        self.add_basic_filters(&mut q);
        q.push(" ORDER BY timestamp ASC LIMIT ");
        q.push_bind(self.limit.unwrap_or(10000) as i64);
        q
    }

    pub async fn fetch_first_n_with_db(
        &self,
        conn: &SqlitePool,
    ) -> Vec<common::Location> {
        let mut q = self.first_n_query(false);
        let query_as = q.build_query_as::<LocationRow>();
        let recs = query_as.fetch_all(conn).await.unwrap();
        to_common_locations(recs)
    }

    pub async fn fetch_first_n(&self) -> Vec<common::Location> {
        let Ok(conn) = get_main_db_pool() else {
            return Vec::new();
        };
        self.fetch_first_n_with_db(&conn).await
    }

    /// Determine the LngLatBounds that encompass the data, ignoring any lnglat
    /// bounding. Consumes self. Assumes decimation.
    fn get_bounds_query(
        &self,
        explain_query_plan: bool,
    ) -> QueryBuilder<'a, Sqlite> {
        let mut q = new_query(explain_query_plan);
        q.push(
            "SELECT MIN(longitude), MIN(latitude),
                    MAX(longitude), MAX(latitude)
                FROM location WHERE 1",
        );
        add_bounds_to_query(&self.bounds, &mut q);
        self.add_basic_filters(&mut q);
        q
    }

    pub async fn fetch_bounds_with_db(
        &self,
        conn: &SqlitePool,
    ) -> Option<LngLatBounds> {
        let mut q = self.get_bounds_query(false);
        let query_as = q.build_query_as::<LngLatBoundsResult>();
        match query_as.fetch_one(conn).await {
            Ok(LngLatBoundsResult(
                Some(swlng),
                Some(swlat),
                Some(nelng),
                Some(nelat),
            )) => Some(LngLatBounds {
                sw: LngLat {
                    lng: swlng,
                    lat: swlat,
                },
                ne: LngLat {
                    lng: nelng,
                    lat: nelat,
                },
            }),
            Ok(_) => None,
            Err(e) => {
                error!("Failed to get bounds for filtered query: {e}");
                None
            }
        }
    }

    pub async fn fetch_bounds(&self) -> Option<LngLatBounds> {
        let conn = get_main_db_pool().ok()?;
        self.fetch_bounds_with_db(&conn).await
    }
}

/// Results from Sqlite MIN/MAX functions can be NULL.
#[derive(FromRow)]
struct LngLatBoundsResult(Option<f64>, Option<f64>, Option<f64>, Option<f64>);

fn to_common_locations(recs: Vec<LocationRow>) -> Vec<common::Location> {
    recs.into_iter().map(|l| l.into()).collect()
}

/// Full Location rows for narrow points, looked up on the UNIQUE
/// timestamp in chunks. Sorted by timestamp; rows that failed to fetch
/// are logged and dropped.
pub async fn fetch_full_locations(
    conn: &SqlitePool,
    points: &[NarrowPoint],
) -> Vec<common::Location> {
    // well under any SQLITE_MAX_VARIABLE_NUMBER build setting
    const CHUNK: usize = 500;
    let mut recs = Vec::with_capacity(points.len());
    for chunk in points.chunks(CHUNK) {
        let mut q = new_query(false);
        q.push("SELECT * FROM location WHERE timestamp IN (");
        let mut sep = q.separated(", ");
        for p in chunk {
            sep.push_bind(p.timestamp.unix_timestamp());
        }
        q.push(")");
        match q.build_query_as::<LocationRow>().fetch_all(conn).await {
            Ok(rows) => recs.extend(rows),
            Err(e) => error!("full location fetch failed: {e}"),
        }
    }
    recs.sort_by_key(|r| r.timestamp);
    to_common_locations(recs)
}

/// The full row at a timestamp (UNIQUE), for showing every field of a
/// narrow point.
pub async fn get_location_at(
    conn: &SqlitePool,
    timestamp: time::OffsetDateTime,
) -> Option<common::Location> {
    sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location WHERE timestamp = ?",
    )
    .bind(timestamp.unix_timestamp())
    .fetch_optional(conn)
    .await
    .map_err(|e| error!("location at timestamp fetch failed: {e}"))
    .ok()
    .flatten()
    .map(Into::into)
}

#[derive(FromRow)]
struct CountResult(i64);

/// Create a new QueryBuilder, optionally adding EXPLAIN QUERY PLAN to the
/// beginning so the query can be passed to explain_query().
fn new_query<'a>(explain_query_plan: bool) -> QueryBuilder<'a, Sqlite> {
    let mut q: QueryBuilder<Sqlite> = QueryBuilder::new("");
    if explain_query_plan {
        q.push("EXPLAIN QUERY PLAN ");
    }
    q
}

/// Take an explainable query and print out the query plan. This is usedful for
/// debugging slow queries, since it shows what indexes are used.
#[allow(unused)]
async fn explain_query(mut q: QueryBuilder<'_, Sqlite>) {
    let query = q.build_query_as::<ExplainQueryPlan>();
    let Ok(conn) = get_main_db_pool() else {
        return;
    };
    let rows = query.fetch_all(&conn).await.unwrap();
    let roots = rows.iter().filter(|x| x.parent == 0);
    for root in roots {
        root.print(&rows, "");
    }
}

/// A row of the output from EXPLAIN QUERY PLAN
#[derive(sqlx::FromRow, Debug)]
struct ExplainQueryPlan {
    id: i64,
    parent: i64,
    #[allow(dead_code)]
    notused: i64,
    detail: String,
}

impl ExplainQueryPlan {
    /// Print the description of the row, then recurse on all children with
    /// increasing indenting. Children are found by looking at rows in the
    /// query plan where the parent is equal to self's id.
    fn print(&self, rows: &Vec<ExplainQueryPlan>, indent: &str) {
        println!("{indent}{}", self.detail);
        let children = rows.iter().filter(|x| x.parent == self.id);
        let new_indent = format!("  {}", indent);
        for c in children {
            c.print(rows, &new_indent)
        }
    }
}

/// Add a starting timestamp constraint to a WHERE clause in a QueryBuilder.
/// Start time is inclusive.
///
/// Assumes a condition has already been added to the WHERE clause, as `AND` is
/// prepended for both the upper and lower bounds on the time filter.
///
/// With TimeHint::AvoidIndex the condition is wrapped in likelihood(X, 1.0)
/// to tell the planner it isn't selective, so the timestamp index isn't
/// used for it. UseIndex leaves the bare comparison, which the planner
/// serves from the timestamp index.
fn add_start_time_to_query(
    start_time: &Option<jiff::Timestamp>,
    query: &mut QueryBuilder<Sqlite>,
    hint: TimeHint,
) {
    if let Some(start) = start_time {
        add_time_condition(query, " AND timestamp >= ", start, hint);
    }
}

fn add_time_condition(
    query: &mut QueryBuilder<Sqlite>,
    cond: &str,
    t: &jiff::Timestamp,
    hint: TimeHint,
) {
    match hint {
        TimeHint::UseIndex => {
            query.push(cond);
            query.push_bind(t.as_second());
        }
        TimeHint::AvoidIndex => {
            query.push(cond.replace("timestamp", "likelihood(timestamp"));
            query.push_bind(t.as_second());
            query.push(", 1.0)");
        }
    }
}

/// Add a ending timestamp constraint to a WHERE clause that already has a
/// condition. End time is exclusive.
fn add_end_time_to_query(
    end_time: &Option<jiff::Timestamp>,
    query: &mut QueryBuilder<Sqlite>,
    hint: TimeHint,
) {
    if let Some(end) = end_time {
        add_time_condition(query, " AND timestamp < ", end, hint);
    }
}

/// Adds a filter condition to a SQL query. The WHERE clause must already have a
/// condition.
fn add_filters_to_query(
    filters: &Option<Vec<Filter>>,
    query: &mut QueryBuilder<Sqlite>,
) {
    if let Some(filters) = filters {
        for filter in filters {
            if filter.enabled {
                query.push(
                    "
                AND ",
                );
                query.push(stream_column_name(filter));
                query.push(" ");
                query.push(op_to_sql(filter));
                query.push(" ");
                query.push_bind(filter.threshold);
            }
        }
    }
}

/// Add a decimation condition to a query.
fn add_decim_to_query(decim: i64, q: &mut QueryBuilder<Sqlite>) {
    // a bound parameter isn't constant-folded, so skip the trivial case
    if decim <= 1 {
        return;
    }
    q.push(" AND id % ");
    q.push_bind(decim);
    q.push(" == 0 ");
}

/// Adds lnglat bound conditions to a SQL query.
///
/// LngLatBounds can "spill over" onto the next left/right map alias if the view
/// includes the antimeridian. This means the bounds can be beyond [-180, 180]
fn add_bounds_to_query(
    bounds: &Option<LngLatBounds>,
    q: &mut QueryBuilder<Sqlite>,
) {
    if let Some(bounds) = bounds {
        q.push(" AND latitude >= ");
        q.push_bind(bounds.sw.lat);
        q.push(
            "
                AND latitude <= ",
        );
        q.push_bind(bounds.ne.lat);
        // whether to create an alias at +/-360 degrees
        let alias_positive = bounds.sw.lng < -180.;
        let alias_negative = bounds.ne.lng > 180.;
        if alias_negative && alias_positive {
            // if we're aliasing on both sides, that means the full lng width
            // of the map is visible and we don't need to have any lng bounds
            return;
        }
        q.push(
            "
                AND (",
        );
        let add_lng_bounds =
            |low: f64, high: f64, q: &mut QueryBuilder<Sqlite>| {
                q.push("(longitude >= ");
                q.push_bind(low);
                q.push(" AND longitude <= ");
                q.push_bind(high);
                q.push(")");
            };
        add_lng_bounds(bounds.sw.lng, bounds.ne.lng, q);
        if alias_positive {
            q.push(
                "
                   OR ",
            );
            add_lng_bounds(bounds.sw.lng + 360., bounds.ne.lng + 360., q);
        }
        if alias_negative {
            q.push(
                "
                   OR ",
            );
            add_lng_bounds(bounds.sw.lng - 360., bounds.ne.lng - 360., q);
        }
        q.push(")");
    }
}

/// INVERTS the condition, since we have filters *hide* data where true.
fn op_to_sql(filter: &Filter) -> &str {
    match filter.op {
        FilterOp::GreaterThan => "<=",
        FilterOp::LessThan => ">=",
        FilterOp::GreatherThanOrEq => "<",
        FilterOp::LessThanOrEq => ">",
        FilterOp::IsEq => "!=",
        FilterOp::IsNotEq => "==",
    }
}

/// Retrieves the column name given a variant of the DataStream enum.
fn stream_column_name(filter: &Filter) -> &str {
    match filter.datastream {
        DataStream::Lat => "latitude",
        DataStream::Lon => "longitude",
        DataStream::HorizAccuracy => "horizontal_accuracy",
        DataStream::Altitude => "msl_altitude",
        DataStream::VertAccuracy => "vertical_accuracy",
        DataStream::Story => "story",
        DataStream::Speed => "speed",
        DataStream::SpeedAccuracy => "speed_accuracy",
        DataStream::Course => "course",
        DataStream::CourseAccuracy => "course_accuracy",
    }
}

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    use common::pin::Pin;
    use pretty_assertions::assert_eq;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

    use crate::app_state::get_derived_state;
    use crate::map::automap::{get_last_automap_update, update_automap};
    use crate::{local::test_setup, paths::get_documents_dir};

    use super::*;

    #[tokio::test]
    async fn test_get_db_pool() {
        test_setup("test_get_db_pool/").await;
        assert!(get_main_db_pool().is_ok());
    }

    /// Generate location data that is different for each idx. SQLite will start
    /// database ids at 1.
    pub fn get_test_data(idx: usize) -> OSLocationData {
        OSLocationData {
            timestamp: idx as i64 * 5,
            latitude: idx as f64,
            longitude: idx as f64,
            horizontal_accuracy: idx as f64,
            msl_altitude: idx as f64,
            ellipsoid_altitude: idx as f64,
            vertical_accuracy: idx as f64,
            story_available: true,
            story: idx as i64,
            speed: idx as f64,
            speed_accuracy: idx as f64,
            course: idx as f64,
            course_accuracy: idx as f64,
            source_info_available: true,
            is_simulated_by_software: true,
            is_produced_by_accessory: false,
        }
    }

    #[tokio::test]
    async fn test_bounded_query() {
        test_setup("test_bounded_query/").await;
        let mut recs = vec![];
        for i in 0..10 {
            recs.push(get_test_data(i));
            log_location(recs[i].clone()).await.unwrap();
        }
        let bounds = LngLatBounds {
            sw: LngLat {
                lng: -10.,
                lat: -5.,
            },
            ne: LngLat { lng: 5.5, lat: 5.5 },
        };
        let records = FilteredQuery::builder()
            .bounds(bounds)
            .decimation_threshold(10000)
            .build()
            .fetch_decimated()
            .await;
        assert_eq!(records.len(), 6);

        let records = FilteredQuery::builder()
            .bounds(bounds)
            .get_adjacent(true)
            .decimation_threshold(10000)
            .build()
            .fetch_decimated()
            .await;
        assert_eq!(records.len(), 7);
    }

    #[tokio::test]
    async fn test_aliased_bounded_query() {
        test_setup("test_aliased_bounded_query/").await;
        let lnglats = [
            (170., 0.0),
            (179., 0.0), // inside
            (179., 40.0),
            (-179., 0.0), // inside
            (-179., 40.0),
            (-170., 0.0),
        ];
        for (i, lnglat) in lnglats.iter().enumerate() {
            let mut r = get_test_data(i);
            r.longitude = lnglat.0;
            r.latitude = lnglat.1;
            log_location(r).await.unwrap();
        }
        let bounds = LngLatBounds {
            sw: LngLat {
                lng: 175.,
                lat: -5.,
            },
            ne: LngLat {
                lng: 185.,
                lat: 5.5,
            },
        };
        let records = FilteredQuery::builder()
            .bounds(bounds)
            .decimation_threshold(10000)
            .build()
            .fetch_decimated()
            .await;
        assert_eq!(records.len(), 2);

        let records = FilteredQuery::builder()
            .bounds(bounds)
            .get_adjacent(true)
            .decimation_threshold(10000)
            .build()
            .fetch_decimated()
            .await;
        assert_eq!(records.len(), 5);
    }

    /// Latitude, longitude of a grid coordinate (inverse of grid_coords).
    fn lnglat_from_grid(gx: u32, gy: u32) -> (f64, f64) {
        let scale = 2f64.powi(grid::GRID_BITS as i32);
        let lng = gx as f64 / scale * 360. - 180.;
        let lat = (std::f64::consts::PI * (1. - 2. * gy as f64 / scale))
            .sinh()
            .atan()
            .to_degrees();
        (lat, lng)
    }

    /// Log `n` points north-east from (0, 0) along the diagonal, one per
    /// 2^14 grid units, each at the centre of its 2^14 cell (so none sit
    /// on a cell boundary at any coarser shift either).
    async fn log_diagonal(n: i64) {
        for i in 0..n {
            let mut loc = get_test_data(i as usize);
            let d = (i as u32) << 14 | 1 << 13;
            let (lat, lng) = lnglat_from_grid(1 << 31 | d, (1 << 31) - d);
            loc.latitude = lat;
            loc.longitude = lng;
            log_location(loc).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_bucketed_query() {
        test_setup("test_bucketed_query/").await;
        // 20 points one 2^14 cell apart; a 2^16 cell holds 4 of them
        log_diagonal(20).await;
        let conn = get_main_db_pool().unwrap();
        let cell = SpatialCell { grid_shift: 16 };

        // over the limit: the most recent point per cell, in time order
        let result = FilteredQuery::builder()
            .decimation_threshold(5)
            .spatial_cell(cell)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(result.spatially_bucketed);
        let timestamps: Vec<_> = result
            .records
            .iter()
            .map(|r| r.timestamp.unix_timestamp())
            .collect();
        assert_eq!(timestamps, vec![3 * 5, 7 * 5, 11 * 5, 15 * 5, 19 * 5]);

        // bounds compose with bucketing: the first 10 points, 3 cells
        let (lat, lng) =
            lnglat_from_grid(1 << 31 | 10 << 14, (1 << 31) - (10 << 14));
        let bounds = LngLatBounds {
            sw: LngLat { lng: 0., lat: 0. },
            ne: LngLat { lng, lat },
        };
        let result = FilteredQuery::builder()
            .bounds(bounds)
            .decimation_threshold(5)
            .spatial_cell(cell)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(result.spatially_bucketed);
        assert_eq!(result.records.len(), 3);

        // under the limit: everything is returned untouched
        let result = FilteredQuery::builder()
            .decimation_threshold(10000)
            .spatial_cell(cell)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(!result.spatially_bucketed);
        assert_eq!(result.records.len(), 20);
    }

    /// field1/field2 fetch the caller's columns through every route: CAST
    /// so INTEGER columns (story) decode as f64 (sqlx 0.8 type-checks
    /// against the value's storage class), NULL when unset, and bucketed
    /// rows take them from the row holding max(timestamp).
    #[tokio::test]
    async fn test_narrow_fields() {
        test_setup("test_narrow_fields/").await;
        // story and speed equal the point's index (timestamp / 5)
        log_diagonal(20).await;
        let conn = get_main_db_pool().unwrap();

        // unset fields come back as None
        let recs = FilteredQuery::builder()
            .decimation_threshold(10000)
            .build()
            .fetch_decimated_with_db(&conn)
            .await;
        assert_eq!(recs.len(), 20);
        assert!(recs
            .iter()
            .all(|r| r.field1.is_none() && r.field2.is_none()));

        // temporal fetch: story (INTEGER) decodes as f64 via the CAST
        let recs = FilteredQuery::builder()
            .decimation_threshold(10000)
            .field1("story")
            .field2("speed")
            .build()
            .fetch_decimated_with_db(&conn)
            .await;
        for r in &recs {
            let i = (r.timestamp.unix_timestamp() / 5) as f64;
            assert_eq!(r.field1, Some(i));
            assert_eq!(r.field2, Some(i));
        }

        // bucketed on both routes: values come from each cell's newest row
        let cell = SpatialCell { grid_shift: 16 };
        let grid_route = FilteredQuery::builder()
            .decimation_threshold(5)
            .spatial_cell(cell)
            .field1("story")
            .build();
        let time_route = FilteredQuery::builder()
            .start(jiff::Timestamp::from_second(0).unwrap())
            .decimation_threshold(5)
            .spatial_cell(cell)
            .field1("story")
            .build();
        for query in [grid_route, time_route] {
            let result = query.fetch_decimated_result_with_db(&conn).await;
            assert!(result.spatially_bucketed);
            let vals: Vec<_> =
                result.records.iter().map(|r| r.field1).collect();
            let expected: Vec<_> = [3., 7., 11., 15., 19.].map(Some).to_vec();
            assert_eq!(vals, expected);
        }
    }

    /// The hard_cap memory backstop binds only when it must: on the cluster
    /// pathology (many occupied cells at a tiny pitch) and on a cap below the
    /// temporal limit (memory-limits.md, "Backend memory backstop").
    #[tokio::test]
    async fn test_hard_cap() {
        test_setup("test_hard_cap/").await;
        // 30 points one cell per point: every point occupies its own
        // cell, so the bucketed path's cell-count bound is no bound
        log_diagonal(30).await;
        let conn = get_main_db_pool().unwrap();
        let cell = SpatialCell { grid_shift: 14 };

        // bucketed, cap binds: the most recent 10 cells, ascending in time
        let result = FilteredQuery::builder()
            .decimation_threshold(5)
            .spatial_cell(cell)
            .hard_cap(10)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(result.spatially_bucketed);
        assert!(result.memory_capped);
        let ts: Vec<_> = result
            .records
            .iter()
            .map(|r| r.timestamp.unix_timestamp())
            .collect();
        assert_eq!(ts, (20i64..30).map(|i| i * 5).collect::<Vec<_>>());

        // bucketed, cap above the cell count: never binds
        let result = FilteredQuery::builder()
            .decimation_threshold(5)
            .spatial_cell(cell)
            .hard_cap(1000)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(result.spatially_bucketed);
        assert!(!result.memory_capped);
        assert_eq!(result.records.len(), 30);

        // temporal, cap below the limit: coarser sample, flagged
        let result = FilteredQuery::builder()
            .decimation_threshold(20)
            .hard_cap(10)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(!result.spatially_bucketed);
        assert!(result.memory_capped);
        assert!(result.records.len() <= 10);

        // temporal, cap above the limit: inert
        let result = FilteredQuery::builder()
            .decimation_threshold(20)
            .hard_cap(10_000)
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(!result.memory_capped);
        assert!(result.records.len() >= 15);
    }

    /// Deterministic synthetic track: a random walk with a few dwell
    /// clusters, around (37.8, -122.5), offshore of SF. Returns the rows
    /// logged.
    async fn log_grid_test_track(n: usize) {
        let mut rng = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng >> 11) as f64 / (1u64 << 53) as f64
        };
        let (mut lat, mut lng) = (37.8, -122.5);
        for i in 0..n {
            let mut loc = get_test_data(i + 1);
            if i % 50 < 30 {
                // dwell: jitter around the current spot
                loc.latitude = lat + (next() - 0.5) * 1e-4;
                loc.longitude = lng + (next() - 0.5) * 1e-4;
            } else {
                lat += (next() - 0.5) * 2e-3;
                lng += (next() - 0.5) * 2e-3;
                loc.latitude = lat;
                loc.longitude = lng;
            }
            loc.horizontal_accuracy = (i % 7) as f64 * 10.;
            log_location(loc).await.unwrap();
        }
    }

    /// Reference: newest id per grid query cell via a plain GROUP BY over
    /// the same grid cells, with the same bounds/time/filter clauses.
    async fn grid_reference(
        conn: &SqlitePool,
        query: &FilteredQuery,
        shift: u32,
    ) -> Vec<i64> {
        let mut q = new_query(false);
        q.push("SELECT id, max(timestamp) FROM location WHERE 1");
        add_bounds_to_query(&query.bounds, &mut q);
        query.add_basic_filters(&mut q);
        q.push(format!(" GROUP BY gy >> {shift}, gx >> {shift}"));
        let mut ids: Vec<i64> = q
            .build_query_as::<(i64, i64)>()
            .fetch_all(conn)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.0)
            .collect();
        ids.sort();
        ids
    }

    /// The loose index scan returns exactly the GROUP BY answer at every
    /// index relationship (matched, coarser query, finer query), with
    /// bounds, a time range, and a filter.
    #[tokio::test]
    async fn test_grid_bucketed_matches_group_by() {
        test_setup("test_grid_bucketed_matches_group_by/").await;
        log_grid_test_track(600).await;
        let conn = get_main_db_pool().unwrap();

        let bounds = LngLatBounds {
            sw: LngLat {
                lng: -122.52,
                lat: 37.78,
            },
            ne: LngLat {
                lng: -122.48,
                lat: 37.82,
            },
        };
        let filter = Filter {
            id: 0,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 45.,
            enabled: true,
        };
        let queries = [
            FilteredQuery::builder().build(),
            FilteredQuery::builder().bounds(bounds).build(),
            FilteredQuery::builder()
                .bounds(bounds)
                .start(jiff::Timestamp::from_second(100 * 5).unwrap())
                .end(jiff::Timestamp::from_second(500 * 5).unwrap())
                .filters(vec![filter])
                .build(),
        ];
        // matched (L12, L10, L8), coarser query than L8/L10/L12, finer
        // than every index
        let shifts = [10, 12, 14, 11, 13, 16, 20, 9, 6];
        for query in &queries {
            for shift in shifts {
                let expected = grid_reference(&conn, query, shift).await;
                assert!(!expected.is_empty());
                let (recs, memory_capped) =
                    query.fetch_bucketed_grid(&conn, shift).await.unwrap();
                assert!(!memory_capped);
                let mut ids: Vec<_> = recs.iter().map(|r| r.id).collect();
                ids.sort();
                assert_eq!(ids, expected, "shift {shift}");
                // ascending time order, as the tileset build expects
                assert!(recs
                    .windows(2)
                    .all(|w| w[0].timestamp < w[1].timestamp));
            }
        }

        // hard_cap keeps the most recent cells and flags truncation
        let query = FilteredQuery::builder().hard_cap(10).build();
        let expected = grid_reference(&conn, &query, 12).await;
        let (recs, memory_capped) =
            query.fetch_bucketed_grid(&conn, 12).await.unwrap();
        assert!(memory_capped);
        assert_eq!(recs.len(), 10);
        let newest: Vec<_> = recs.iter().map(|r| r.id).collect();
        let mut top: Vec<_> = expected.clone();
        top.sort_by_key(|id| std::cmp::Reverse(*id)); // ids are time-ordered
        top.truncate(10);
        top.sort();
        assert_eq!(newest, top);

        // the full decimated path takes the grid route when bucketing
        let result = FilteredQuery::builder()
            .decimation_threshold(5)
            .spatial_cell(SpatialCell::from_zoom(12., 0.5))
            .build()
            .fetch_decimated_result_with_db(&conn)
            .await;
        assert!(result.spatially_bucketed);
        let expected = grid_reference(
            &conn,
            &FilteredQuery::builder().build(),
            grid::query_shift(12., 0.5),
        )
        .await;
        assert_eq!(result.records.len(), expected.len());
    }

    /// With a time range under TIME_PATH_MAX_ROWS the query routes through
    /// the timestamp index, and bucketing there gives exactly the grid
    /// walk's answer (same cells, same bounds/filters).
    #[tokio::test]
    async fn test_time_route_matches_grid_walk() {
        test_setup("test_time_route_matches_grid_walk/").await;
        log_grid_test_track(600).await;
        let conn = get_main_db_pool().unwrap();
        let bounds = LngLatBounds {
            sw: LngLat {
                lng: -122.52,
                lat: 37.78,
            },
            ne: LngLat {
                lng: -122.48,
                lat: 37.82,
            },
        };
        let filter = Filter {
            id: 0,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 45.,
            enabled: true,
        };
        let queries = [
            FilteredQuery::builder()
                .start(jiff::Timestamp::from_second(100 * 5).unwrap())
                .build(),
            FilteredQuery::builder()
                .bounds(bounds)
                .end(jiff::Timestamp::from_second(500 * 5).unwrap())
                .build(),
            FilteredQuery::builder()
                .bounds(bounds)
                .start(jiff::Timestamp::from_second(100 * 5).unwrap())
                .end(jiff::Timestamp::from_second(500 * 5).unwrap())
                .filters(vec![filter])
                .build(),
        ];
        for query in &queries {
            let rows = query.route_inputs(&conn).await;
            assert!(rows.is_some_and(|n| n <= TIME_PATH_MAX_ROWS));
            for shift in [10, 12, 14, 16, 20, 6] {
                let expected = grid_reference(&conn, query, shift).await;
                assert!(!expected.is_empty());
                let (recs, memory_capped) = query
                    .fetch_bucketed(&conn, shift, TimeHint::UseIndex)
                    .await;
                assert!(!memory_capped);
                let mut ids: Vec<_> = recs.iter().map(|r| r.id).collect();
                ids.sort();
                assert_eq!(ids, expected, "shift {shift}");
                assert!(recs
                    .windows(2)
                    .all(|w| w[0].timestamp < w[1].timestamp));
            }
        }
        // no time range: no row count
        let query = FilteredQuery::builder().bounds(bounds).build();
        assert_eq!(query.route_inputs(&conn).await, None);
        // the full decimated path on the time route
        let shift = grid::query_shift(12., 0.5);
        let query = FilteredQuery::builder()
            .bounds(bounds)
            .start(jiff::Timestamp::from_second(100 * 5).unwrap())
            .decimation_threshold(5)
            .spatial_cell(SpatialCell::from_zoom(12., 0.5))
            .build();
        let result = query.fetch_decimated_result_with_db(&conn).await;
        assert!(result.spatially_bucketed);
        let expected = grid_reference(&conn, &query, shift).await;
        assert_eq!(result.records.len(), expected.len());
    }

    /// Temporal decimation through the grid and through the timestamp
    /// index both give the plain scan's count and rows.
    #[tokio::test]
    async fn test_temporal_routes_match_scan() {
        test_setup("test_temporal_routes_match_scan/").await;
        log_grid_test_track(600).await;
        let conn = get_main_db_pool().unwrap();
        let bounds = LngLatBounds {
            sw: LngLat {
                lng: -122.52,
                lat: 37.78,
            },
            ne: LngLat {
                lng: -122.48,
                lat: 37.82,
            },
        };
        // a narrow rect, so the filter level is the finest and the edge
        // cells matter
        let narrow = LngLatBounds {
            sw: LngLat {
                lng: -122.501,
                lat: 37.799,
            },
            ne: LngLat {
                lng: -122.499,
                lat: 37.801,
            },
        };
        let filter = Filter {
            id: 0,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 45.,
            enabled: true,
        };
        let time = jiff::Timestamp::from_second(100 * 5).unwrap();
        let queries = [
            FilteredQuery::builder().build(),
            FilteredQuery::builder().bounds(bounds).build(),
            FilteredQuery::builder().bounds(narrow).build(),
            FilteredQuery::builder()
                .bounds(bounds)
                .filters(vec![filter])
                .build(),
            FilteredQuery::builder().bounds(bounds).start(time).build(),
            FilteredQuery::builder()
                .bounds(narrow)
                .end(time)
                .filters(vec![filter])
                .build(),
        ];
        for query in &queries {
            let mut q = query.count_query(false, 1, TimeHint::AvoidIndex);
            let expected: i64 =
                q.build_query_scalar().fetch_one(&conn).await.unwrap();
            assert!(expected > 0);
            // bounded and exact grid counts agree with the scan
            let n = query.fetch_count_grid(&conn, None, true).await.unwrap();
            assert_eq!(n, expected);
            let n = query
                .fetch_count_grid(&conn, Some(expected), true)
                .await
                .unwrap();
            assert_eq!(n, expected);
            let n = query.fetch_count_grid(&conn, Some(2), true).await.unwrap();
            assert_eq!(n, expected.min(3));
            // the unfiltered count is the all-time count in view
            let mut q = FilteredQuery::builder()
                .maybe_bounds(query.bounds)
                .build()
                .count_query(false, 1, TimeHint::AvoidIndex);
            let all: i64 =
                q.build_query_scalar().fetch_one(&conn).await.unwrap();
            let n = query.fetch_count_grid(&conn, None, false).await.unwrap();
            assert_eq!(n, all);
            // time route decimation agrees with the scan
            let (a, _) = query.fetch_decim(&conn, TimeHint::UseIndex).await;
            let (b, _) = query.fetch_decim(&conn, TimeHint::AvoidIndex).await;
            assert_eq!(a, b);
            for decim in [1, 3, 7] {
                let expected: Vec<_> = query
                    .fetch_bounded(&conn, decim, TimeHint::AvoidIndex)
                    .await
                    .iter()
                    .map(|r| r.id)
                    .collect();
                for recs in [
                    query.fetch_bounded_grid(&conn, decim).await.unwrap(),
                    query.fetch_bounded(&conn, decim, TimeHint::UseIndex).await,
                ] {
                    let got: Vec<_> = recs.iter().map(|r| r.id).collect();
                    assert_eq!(got, expected, "decim {decim}");
                }
            }
        }
        // the full path returns the same rows as before
        let query = FilteredQuery::builder()
            .bounds(bounds)
            .decimation_threshold(50)
            .build();
        let recs = query.fetch_decimated_with_db(&conn).await;
        let (decim, _) = query.fetch_decim(&conn, TimeHint::AvoidIndex).await;
        let old = query
            .fetch_bounded(&conn, decim, TimeHint::AvoidIndex)
            .await;
        assert_eq!(recs.len(), old.len());
    }

    /// Every seek in the grid query goes through a level index (INDEXED BY
    /// makes a fallback an error, this pins the plan shape too), and the
    /// only table access is the final rowid join.
    #[tokio::test]
    async fn test_grid_query_plan() {
        test_setup("test_grid_query_plan/").await;
        log_grid_test_track(20).await;
        let conn = get_main_db_pool().unwrap();
        let bounds = LngLatBounds {
            sw: LngLat {
                lng: -122.52,
                lat: 37.78,
            },
            ne: LngLat {
                lng: -122.48,
                lat: 37.82,
            },
        };
        let query = FilteredQuery::builder()
            .bounds(bounds)
            .end(jiff::Timestamp::from_second(1000).unwrap())
            .build();
        for (shift, level) in [(12, 10), (13, 10), (16, 8), (6, 8)] {
            let rect = query.grid_rects().remove(0);
            let q = query.grid_bucketed_query(&rect, shift);
            let sql = q.sql().to_string();
            println!("--- shift {shift} ---\n{sql}");
            let plan: Vec<(i64, i64, i64, String)> =
                sqlx::query_as(&format!("EXPLAIN QUERY PLAN {sql}"))
                    .bind(1000i64)
                    .fetch_all(&conn)
                    .await
                    .unwrap();
            let steps: Vec<&str> = plan.iter().map(|p| p.3.as_str()).collect();
            let idx = grid::index_name(level);
            assert!(steps.iter().any(|st| st.contains(&idx)), "{steps:?}");
            // table access only via the level index or the final winner
            // join: the timestamp autoindex in the walk form, rowid in the
            // filter form
            for st in steps.iter().filter(|st| st.contains("location")) {
                assert!(
                    st.contains(&idx)
                        || st.contains("sqlite_autoindex_location")
                        || st.contains("INTEGER PRIMARY KEY"),
                    "unexpected table access: {st}"
                );
            }
        }
        // temporal-mode filter: one index seek per cell row of the CTE
        let rect = query.grid_rects().remove(0);
        let mut q = query.grid_filter_query(&rect, "count(*)", "");
        query.add_basic_filters(&mut q);
        let sql = q.sql().to_string();
        let plan: Vec<(i64, i64, i64, String)> =
            sqlx::query_as(&format!("EXPLAIN QUERY PLAN {sql}"))
                .bind(1000i64)
                .fetch_all(&conn)
                .await
                .unwrap();
        let steps: Vec<&str> = plan.iter().map(|p| p.3.as_str()).collect();
        let level = grid::filter_level(rect.x1 - rect.x0, rect.y1 - rect.y0);
        let idx = grid::index_name(level);
        assert!(
            steps
                .iter()
                .any(|st| st.contains(&idx) && st.contains("cy")),
            "{steps:?}"
        );
        assert!(
            steps.iter().all(|st| !st.contains("SCAN location")),
            "{steps:?}"
        );
    }

    /// Rows inserted without grid coords (an import, or a pre-migration
    /// database) are backfilled in batches, resumably.
    #[tokio::test]
    async fn test_grid_backfill() {
        test_setup("test_grid_backfill/").await;
        let conn = get_main_db_pool().unwrap();
        for i in 0..50 {
            let idx = i as f64;
            sqlx::query(
                "INSERT INTO location (timestamp, latitude, longitude, horizontal_accuracy) VALUES (?, ?, ?, 1.0)",
            )
            .bind(i * 5)
            .bind(37. + idx * 1e-3)
            .bind(-122. + idx * 1e-3)
            .execute(&conn)
            .await
            .unwrap();
        }
        log_location(get_test_data(1000)).await.unwrap();
        let mut reports = Vec::new();
        let filled = grid::backfill(&conn, |n, total| reports.push((n, total)))
            .await
            .unwrap();
        assert_eq!(filled, 50);
        assert_eq!(reports, vec![(50, 50)]);
        let nulls: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM location WHERE gx IS NULL OR gy IS NULL",
        )
        .fetch_one(&conn)
        .await
        .unwrap();
        assert_eq!(nulls, 0);
        // backfilled coords agree with insert-time coords
        let rows = sqlx::query_as::<_, (f64, f64, i64, i64)>(
            "SELECT latitude, longitude, gx, gy FROM location",
        )
        .fetch_all(&conn)
        .await
        .unwrap();
        for (lat, lng, gx, gy) in rows {
            let (ex, ey) = grid::grid_coords(&LngLat { lng, lat });
            assert_eq!((gx, gy), (ex as i64, ey as i64));
        }
        // nothing left to fill
        let filled = grid::backfill(&conn, |_, _| {}).await.unwrap();
        assert_eq!(filled, 0);
    }

    /// The startup checkpoint after an open (migrations, backfill, index
    /// builds on several pool connections) completes: nothing in the pool
    /// holds a read snapshot that would block TRUNCATE.
    #[tokio::test]
    async fn test_checkpoint_after_open() {
        test_setup("test_checkpoint_after_open/").await;
        let conn = get_main_db_pool().unwrap();
        for i in 0..100 {
            log_location(get_test_data(i)).await.unwrap();
        }
        sqlx::query("VACUUM;").execute(&conn).await.unwrap();
        checkpoint_wal(&conn).await.unwrap();
    }

    #[tokio::test]
    async fn test_records_time_range() {
        test_setup("test_records_time_range/").await;

        // 5 and 10 seconds past the epoch
        log_location(get_test_data(1)).await.unwrap(); // timestamp: 5
        log_location(get_test_data(2)).await.unwrap(); // timestamp: 10
        let start = jiff::Timestamp::from_second(3).unwrap();
        let end = jiff::Timestamp::from_second(7).unwrap();
        // get the first
        let records = FilteredQuery::builder()
            .start(start)
            .end(end)
            .limit(1_000)
            .build()
            .fetch_first_n()
            .await;
        assert_eq!(records.len(), 1);
        // small integer floats can be exactly compared
        assert!(records[0].latitude == 1.0);
        assert!(records[0].longitude == 1.0);
    }

    /// Test that importing records into the database works as expected.
    #[tokio::test]
    async fn test_db_import() {
        test_setup("test_db_import/").await;

        // log some data in our main database
        log_location(get_test_data(1)).await.unwrap(); // timestamp: 5
        log_location(get_test_data(2)).await.unwrap(); // timestamp: 10

        // turn on automap to test that the last_updated time gets reset
        // appropriately if data is added which is earlier
        AppState::global().persistent.lock().unwrap().front =
            Some(Default::default());
        AppState::global()
            .persistent
            .lock()
            .unwrap()
            .front
            .as_mut()
            .unwrap()
            .map
            .style
            .automap = true;
        assert_eq!(get_last_automap_update().unix_timestamp(), 0);
        update_automap().await;
        assert_eq!(get_last_automap_update().unix_timestamp(), 10);

        // create a new database with a new record and a duplicate
        let docdir = get_documents_dir();
        let db_name = "to_import.db";
        let db_path = docdir.join(db_name);
        let db_url = format!("sqlite://{}", db_path.display());
        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let import_conn = SqlitePool::connect_with(opt).await.unwrap();
        MIGRATOR.run(&import_conn).await.unwrap();

        sqlx::query(
            "INSERT INTO location (
                timestamp,
                latitude, longitude, horizontal_accuracy
            )
            VALUES
                (?,?,?,?)",
        )
        .bind(5) // duplicate timestamp
        .bind(-1.0)
        .bind(-1.0)
        .bind(1.0)
        .execute(&import_conn)
        .await
        .unwrap();

        // new data
        sqlx::query(
            "INSERT INTO location (
                timestamp,
                latitude, longitude, horizontal_accuracy
            )
            VALUES
                (?,?,?,?)",
        )
        .bind(8) // unique timestamp
        .bind(4.0)
        .bind(4.0)
        .bind(4.0)
        .execute(&import_conn)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO pins (
                lng, lat, name, icon, lists, tags
            )
            VALUES
                (?,?,?,?,?,?)",
        )
        .bind(-120.0)
        .bind(37.0)
        .bind("My Pin")
        .bind("a")
        .bind("[]")
        .bind("[]")
        .execute(&import_conn)
        .await
        .unwrap();

        import_conn.close().await;

        // import records
        import_database_records(db_path).await;

        let start = jiff::Timestamp::from_second(0).unwrap();
        let end = jiff::Timestamp::from_second(20).unwrap();
        let records = FilteredQuery::builder()
            .start(start)
            .end(end)
            .limit(1_000)
            .build()
            .fetch_first_n()
            .await;

        assert_eq!(records.len(), 3);
        assert_eq!(
            records,
            vec![
                get_test_data(1).into(),
                common::Location {
                    timestamp: time::OffsetDateTime::from_unix_timestamp(8)
                        .unwrap(),
                    latitude: 4.0,
                    longitude: 4.0,
                    horizontal_accuracy: 4.0,
                    msl_altitude: None,
                    ellipsoid_altitude: None,
                    vertical_accuracy: None,
                    story: None,
                    speed: None,
                    speed_accuracy: None,
                    course: None,
                    course_accuracy: None,
                    is_simulated_by_software: None,
                    is_produced_by_accessory: None,
                    was_imported: true,
                },
                get_test_data(2).into(),
            ]
        );
        // automap time should have gone back from 10 to 8
        assert_eq!(get_last_automap_update().unix_timestamp(), 8);
        update_automap().await;
        assert_eq!(get_last_automap_update().unix_timestamp(), 10);

        // check pins were imported
        let pins = get_derived_state(|s| s.pins.clone());
        assert_eq!(
            pins,
            vec![Pin {
                id: Some(1),
                lnglat: LngLat {
                    lng: -120.0,
                    lat: 37.0,
                },
                name: "My Pin".into(),
                icon: "a".into(),
                lists: vec![],
                tags: vec![],
                boundary: None,
            }]
        );
    }

    /// Test that migrating the database works, and that the data persists.
    #[tokio::test]
    async fn test_unique_timestamp_migration() {
        test_setup("test_unique_timestamp_migration/").await;

        let docdir = get_documents_dir();
        let db_name = "to_migrate.db";
        let db_url = format!("sqlite://{}", docdir.join(db_name).display());

        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let conn = SqlitePool::connect_with(opt.clone()).await.unwrap();

        let all_migrations = &MIGRATOR;
        println!("Have {} migrations", all_migrations.iter().len());
        let first_migration =
            MyMigration(all_migrations.iter().next().unwrap().clone());
        let first_migrator = sqlx::migrate::Migrator::new(&first_migration)
            .await
            .unwrap();

        first_migrator.run(&conn).await.unwrap();

        let data = LocationRow {
            id: 0,
            timestamp: 10,
            latitude: 37.,
            longitude: -122.,
            horizontal_accuracy: 5.2,
            msl_altitude: None,
            ellipsoid_altitude: None,
            vertical_accuracy: None,
            story: None,
            speed: Some(4.),
            speed_accuracy: None,
            course: Some(180.),
            course_accuracy: None,
            is_simulated_by_software: None,
            is_produced_by_accessory: None,
            was_imported: false,
        };
        sqlx::query(
            "INSERT INTO location
        (lat, lon, accuracy, speed, course, timestamp)
        VALUES
        (?,?,?,?,?,?)",
        )
        .bind(data.latitude)
        .bind(data.longitude)
        .bind(data.horizontal_accuracy)
        .bind(data.speed.unwrap())
        .bind(data.course.unwrap())
        .bind(data.timestamp)
        .execute(&conn)
        .await
        .unwrap();

        // show_migrations(&conn).await;
        all_migrations.run(&conn).await.unwrap();
        // show_migrations(&conn).await;

        // close and reopen the pool so we definitely get the new schema
        conn.close().await;
        let conn = SqlitePool::connect_with(opt).await.unwrap();

        // check we can get the data back out
        let retrieved = sqlx::query_as::<_, LocationRow>(
            "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_all(&conn)
        .await
        .unwrap();

        // compare with common::Location to ignore the id
        let res: common::Location = retrieved[0].clone().into();
        let exp: common::Location = data.into();
        assert_eq!(exp, res);
    }

    /// # Test how costly nulls really are
    ///
    /// Primary finding from this test: Use the vacuum command to rebuild the
    /// database and shed unused pages, especially after migrations that involve
    /// copying all the data to a new table.
    ///
    /// Expected size of original: 7 * 8 * 100_000 = 5.6 MB
    /// Expected size of final: (7 * 8 + 9) * 100_000 = 6.5 MB
    /// Expected growth rate: 16%
    ///
    /// Observed size of original: 6.3 MB
    /// Observed size of final: 8.76 MB
    /// Observed growth rate: 39%
    ///
    /// Not sure why it's so much larger in the test.. The real data matches
    /// expected calculations much better.
    ///
    /// # Findings from real data (put here since it's the same subject area)
    ///
    /// Examining migration #2 (unique_timestamp) -> #4 (was_imported) on real
    /// data exported from the app.
    ///
    /// Results on 6/27 export, which was up to the unique_timestamp migration.
    ///         0. Original: 9.3 MB
    ///         1. Vacuumed: 9.1 MB
    ///         2. Vacuumed and then migrated: 19.5 MB
    ///         3. Vacuumed, migrated, and vacuumed: 10.2 MB
    /// 1 -> 3 should accurately show the null cost. Observed growth: 12%
    /// Calculation:
    ///         - We had 7 required columns before, 8 bytes each
    ///         - After we have 8 new nullable columns, but previously invalid
    ///                 speed and course data is now marked as NULL
    ///         - There were 132400 total records, 10170 with NULL speed and
    ///                 19307 with NULL course data
    ///         - Expected size of original: 7 * 8 * 132400 = 7.41 MB
    ///         - Expected size of final:
    ///                 (7 * 8 + 9 * 1) * 132400 - 10170 * 7 - 19307 * 7
    ///                 = 8.40 MB
    ///         - Expected growth: 13%
    ///         - Wow it actually matches to about 1%. Yay!
    ///
    /// With the new columns, here is the expected increase in each row's
    /// storage, assuming high accuracy GPS data:
    ///         With high accuracy GPS data:
    ///                 - before: 7 * 8 = 56 B
    ///                 - after: 12 * 8 + 4 = 100 B
    ///                 - increase in storage use: 79%
    ///         With low accuracy GPS data (no speed, course, vertical_accuracy)
    ///                 - after: 7 * 8 + 9 = 65 B
    ///                 - increase in storage use: 16%
    ///
    /// Expected storage usage rate:
    ///         - 10 MB * 1.79 / 3 months
    ///                 = 71 MB / year
    ///                 = 710 MB / decade (what! actually not very much)
    ///         - Of course, this depends on movement pattern. Someone who moves
    ///                 for a living (e.g. delivery driver), would have a
    ///                 storage growth rate that is probably 10x this or more.
    // #[tokio::test] // uncomment to run the test
    #[allow(dead_code)]
    async fn test_null_size() {
        test_setup("test_null_size/").await;

        let docdir = get_documents_dir();
        let db_name = "to_migrate.db";
        let db_path = docdir.join(db_name);
        let db_url = format!("sqlite://{}", db_path.display());

        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let conn = SqlitePool::connect_with(opt.clone()).await.unwrap();

        println!("Run dir: {}", std::env::current_dir().unwrap().display());
        let all_migrations = &MIGRATOR;
        println!("Have {} migrations", all_migrations.iter().len());
        let first_migration =
            MyMigration(all_migrations.iter().next().unwrap().clone());
        let first_migrator = sqlx::migrate::Migrator::new(&first_migration)
            .await
            .unwrap();

        first_migrator.run(&conn).await.unwrap();

        for _ in 0..100000 {
            sqlx::query(
                "INSERT INTO location
            (lat, lon, accuracy, speed, course, timestamp)
            VALUES
            (?,?,?,?,?,?)",
            )
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<i64>())
            .execute(&conn)
            .await
            .unwrap();
        }
        let get_size = || async {
            sqlx::query("PRAGMA wal_checkpoint(FULL); VACUUM;")
                .execute(&conn)
                .await
                .unwrap();
            let f =
                std::fs::File::open(format!("{}", db_path.display())).unwrap();
            println!("size: {}", f.metadata().unwrap().len());
        };
        get_size().await;

        // show_migrations(&conn).await;
        all_migrations.run(&conn).await.unwrap();
        // show_migrations(&conn).await;

        get_size().await;

        conn.close().await;
        assert_eq!(1, 0); // fail the test deliberately
    }

    /// Debugging helpers for showing the state of the database migrations table
    #[allow(dead_code)]
    async fn show_migrations(conn: &SqlitePool) {
        let retrieved =
            sqlx::query_as::<_, MigrationRow>("SELECT * FROM _sqlx_migrations")
                .fetch_all(conn)
                .await
                .unwrap();
        println!("{:?}", &retrieved);
    }

    #[allow(dead_code)]
    #[derive(Clone, FromRow, Debug)]
    struct MigrationRow {
        pub version: i64,
        pub description: String,
        pub installed_on: time::PrimitiveDateTime,
        pub success: bool,
        pub checksum: Vec<u8>,
        pub execution_time: i64,
    }

    use futures_core::future::BoxFuture;
    use sqlx::error::BoxDynError;
    use sqlx::migrate::{Migration, MigrationSource};

    // Single migration that can be applied.
    #[derive(Debug, Clone)]
    struct MyMigration(Migration);

    impl<'s> MigrationSource<'s> for &'s MyMigration {
        fn resolve(self) -> BoxFuture<'s, Result<Vec<Migration>, BoxDynError>> {
            Box::pin(async move { Ok(vec![self.0.clone()]) })
        }
    }
}
