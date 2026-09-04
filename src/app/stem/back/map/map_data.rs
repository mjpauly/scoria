//! Runs the map data query and builds the per-mount vector-tile working
//! sets (mvt.rs) that maplibre plots.
//!
//! To debug slow queries, use the ExplainQuery created by explain_decimate().
//! ```
//! // prints the query and the query plan
//! database::FilteredQuery::new()
//!     .time_range(map_state.time_range)
//!     .filters(map_state.filters.clone())
//!     .bounds(bounds)
//!     .get_adjacent(make_lines)
//!     .explain_decimate(decimation_threshold(&map_state.style))
//!     .explain()
//!     .await;
//! ```

use std::collections::BTreeMap;

use common::cmaps::CmapParams;
use common::mounted::{EnabledDBs, MountID};
use common::view_position::LngLatBounds;
use sqlx::SqlitePool;

use crate::app_state::{get_back_state, get_front_state, set_back_state};
use crate::core::new_data_is_visible;
use crate::database::{get_db_for_id, NarrowPoint};
use crate::map::coords::TileXYZ;
use crate::map::mvt;
use crate::metrics::color::field1_column;
use crate::metrics::color::get_cmap_data;
use crate::metrics::dashboard::update_timeseries_plot_data;
use crate::tz::datetime_fn_infallible;
use crate::{app_state::AppState, database, ws_session};
use common::{
    cmaps,
    state::{MapState, PersistedRoute},
    LngLat, Location, ToFront,
};

/// Point count above which spatial mode buckets the result. Below it
/// everything is fetched as-is (with lines); the count is small enough
/// that bucketing at any pitch would change little but the lines.
/// Independent of the temporal max-points knob, which spatial mode hides.
pub const SPATIAL_BUCKETING_TRIGGER: u64 = 10_000;

/// The point count the map query decimates down to. In temporal mode it's
/// the user's max-points setting: if the query result is larger, we
/// decimate (select every nth) by a factor large enough to get under it.
/// In spatial mode it only decides when bucketing kicks in
/// (SPATIAL_BUCKETING_TRIGGER). Overridable with the
/// STEM_DECIMATION_THRESHOLD env var so the memory probe harness can load
/// large synthetic datasets (doc/decimation/memory-limits.md).
pub fn decimation_threshold(style: &common::map_style::MapStyle) -> u64 {
    static OVERRIDE: std::sync::OnceLock<Option<u64>> =
        std::sync::OnceLock::new();
    if let Some(t) = OVERRIDE.get_or_init(|| {
        std::env::var("STEM_DECIMATION_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
    }) {
        return *t;
    }
    match style.decimation_mode {
        common::map_style::DecimationMode::Temporal => {
            style.temporal_max_points.points()
        }
        common::map_style::DecimationMode::Spatial => SPATIAL_BUCKETING_TRIGGER,
    }
}

/// Never-bind backend memory backstop, as a per-mount cap on fetched
/// points (doc/decimation/memory-limits.md, "Backend memory backstop").
/// The tileset working sets are the N-scaling backend memory cost, and backend
/// OOM kills the app process, which no reload handler can save; latency makes
/// any N near this cap miserable long before memory does, so it exists only for
/// pathologies like a huge import or a multi-mount pileup.
///
/// budget = 1/4 of physical RAM split across mounts, divided by the live
/// tileset bytes-per-point of the last query when one is available
/// (approx_bytes counts Vec capacity and rebuilds briefly hold old + new
/// tilesets, so the ~80 B/pt constant is intuition only). The floor is
/// only a sanity guard against a pathological measured rate; the
/// backstop sitting below a large user threshold is legitimate -- that's
/// it binding, flagged as memory_capped.
pub fn backend_backstop_points(n_mounts: usize) -> u64 {
    const FALLBACK_BYTES_PER_POINT: u64 = 80;
    let rate = get_back_state(|s| s.last_map_query)
        .filter(|s| s.n_points >= 1_000 && s.tileset_bytes > 0)
        .map(|s| s.tileset_bytes / s.n_points as u64)
        .unwrap_or(FALLBACK_BYTES_PER_POINT)
        .max(FALLBACK_BYTES_PER_POINT);
    let budget = physical_memory_bytes() / 4;
    (budget / rate / n_mounts.max(1) as u64).max(100_000)
}

/// Physical memory of the device, or a conservative weak-device fallback.
pub fn physical_memory_bytes() -> u64 {
    static MEM: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *MEM.get_or_init(|| {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            use std::os::raw::{c_char, c_int, c_void};
            extern "C" {
                fn sysctlbyname(
                    name: *const c_char,
                    oldp: *mut c_void,
                    oldlenp: *mut usize,
                    newp: *mut c_void,
                    newlen: usize,
                ) -> c_int;
            }
            let mut size: u64 = 0;
            let mut len = std::mem::size_of::<u64>();
            let name = b"hw.memsize\0";
            let ret = unsafe {
                sysctlbyname(
                    name.as_ptr() as *const c_char,
                    &mut size as *mut u64 as *mut c_void,
                    &mut len,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if ret == 0 && size > 0 {
                return size;
            }
        }
        #[cfg(any(target_os = "android", target_os = "linux"))]
        {
            // "MemTotal:  3882924 kB"
            let meminfo = std::fs::read_to_string("/proc/meminfo");
            if let Some(kb) = meminfo.ok().and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemTotal:"))?
                    .split_whitespace()
                    .nth(1)?
                    .parse::<u64>()
                    .ok()
            }) {
                return kb * 1024;
            }
        }
        2 * 1024 * 1024 * 1024 // assume a weak device
    })
}

/// Physical footprint of this process (dirty + compressed), the number the
/// iOS per-process jetsam limit is enforced against. 0 if unavailable.
pub fn process_footprint_bytes() -> u64 {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use std::os::raw::{c_int, c_uint};
        // task_vm_info_data_t through phys_footprint (TASK_VM_INFO_REV1);
        // task_info fills as many fields as the count passed in asks for.
        #[repr(C)]
        #[derive(Default)]
        struct TaskVmInfo {
            virtual_size: u64,
            region_count: i32,
            page_size: i32,
            resident_size: u64,
            resident_size_peak: u64,
            device: u64,
            device_peak: u64,
            internal: u64,
            internal_peak: u64,
            external: u64,
            external_peak: u64,
            reusable: u64,
            reusable_peak: u64,
            purgeable_volatile_pmap: u64,
            purgeable_volatile_resident: u64,
            purgeable_volatile_virtual: u64,
            compressed: u64,
            compressed_peak: u64,
            compressed_lifetime: u64,
            phys_footprint: u64,
        }
        extern "C" {
            fn mach_task_self() -> c_uint;
            fn task_info(
                task: c_uint,
                flavor: c_int,
                info: *mut TaskVmInfo,
                count: *mut c_uint,
            ) -> c_int;
        }
        const TASK_VM_INFO: c_int = 22;
        let mut info = TaskVmInfo::default();
        let mut count = (std::mem::size_of::<TaskVmInfo>()
            / std::mem::size_of::<c_uint>()) as c_uint;
        let ret = unsafe {
            task_info(mach_task_self(), TASK_VM_INFO, &mut info, &mut count)
        };
        if ret == 0 {
            return info.phys_footprint;
        }
        0
    }
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        // VmRSS is the closest cheap analogue on Linux/Android
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))?
                    .split_whitespace()
                    .nth(1)?
                    .parse::<u64>()
                    .ok()
            })
            .map(|kb| kb * 1024)
            .unwrap_or(0)
    }
}

/// View window expansion in screen pixels for fetching from the database.
/// We fetch data points outside the window since markers have some width and
/// it's nice to not have them suddenly pop in only when their center is inside
/// the view window. Fixed at the maximum marker radius, since we don't
/// re-fetch when the marker size changes.
pub const BOUND_EXPANSION_PX: f64 = common::map_style::MARKER_SIZE_MAX;

/// Determine if the map state/style is different in a way that means we should
/// update the map data.
fn map_state_is_different(prev: &MapState, curr: &MapState) -> bool {
    prev.time_range != curr.time_range
        || prev.filters != curr.filters
        || prev.view_pos != curr.view_pos
        || prev.style.colored_datastream != curr.style.colored_datastream
        // If marker of line size were zero previously, the old tileset may not
        // have the data, so we should update. Going from visibile to not
        // visible doesn't require an update and looks smoother if updating the
        // size sliders rapidly, so we'll save compute when next updating
        || ((prev.style.marker_size == 0.0) && (curr.style.marker_size != 0.0))
        || ((prev.style.line_size == 0) && (curr.style.line_size != 0))
        || (prev.style.hide_points_outside_viewbounds != curr.style.hide_points_outside_viewbounds)
        // requery if the spatial decimation cell would change (marker size,
        // colored datastream, plot tab, or the setting itself)
        || spatial_cell_for(prev) != spatial_cell_for(curr)
        // requery if the temporal threshold changes
        || decimation_threshold(&prev.style)
            != decimation_threshold(&curr.style)
        // requery if the contrast trim baked into the tile colors changes
        // (the reserve setting, or a basemap change crossing light/dark)
        || prev.style.cmap_trim() != curr.style.cmap_trim()
        || prev.timeline_config != curr.timeline_config
}

/// Determine if we should update the map data
///
/// force_update is used when the app is foregrounded, since data may have come
/// in while the app was in the background, or when actions happen which very
/// likely change the displayed data, like deleting the selected points.
///
/// update conditions:
/// force_update
/// || (the route is on the analyze tab
///     && (there is new data in time range
///         || the map state/style is different from before))
async fn should_update_map_data(
    new_data: Option<Location>,
    force_update: bool,
) -> Option<MapState> {
    // get the map configuration state
    let app_state = AppState::global();
    let mut prev_map_data_guard =
        app_state.map_data.prev_map_state.lock().await;
    let persistent_guard = app_state.persistent.lock().unwrap();
    // the '?' operator returns None if the frontend hasn't been initialized yet
    let map_state = &persistent_guard.front.as_ref()?.map;
    // 'if' blocks test if we should NOT update (passed by returning None)
    if !force_update {
        if let Some(p) = persistent_guard.front.as_ref() {
            if p.route != PersistedRoute::Analyze {
                // not looking at the map, don't update data
                return None;
            }
        } else {
            // No frontend, shouldn't happen if force updated
            return None;
        }
        let new_data_visible = new_data
            .map(|l| new_data_is_visible(&l, map_state, true))
            .unwrap_or(false); // if no new data, it's not going to be visible
        let map_state_different = prev_map_data_guard
            .as_ref()
            .map(|prev_state| map_state_is_different(prev_state, map_state))
            .unwrap_or(true); // if no prev map data, assume map stat is
                              // different
        if !(new_data_visible || map_state_different) {
            // same map state and no new data, don't bother updating
            return None;
        }
    }
    // -> Should update if we get here <-
    // store the current state as the previous state
    *prev_map_data_guard = Some(map_state.clone());
    Some(map_state.clone())
}

/// Query and rebuild each mount's tile working set. Doesn't necessarily
/// update; that is determined by should_update_map_data().
///
/// This function is called whenever the front state changes, new location data
/// is logged while the frontend is active, or it's force updated.
///
/// 'new_data' indicates if this is triggered by new location data as opposed to
/// a change to the map's style
pub async fn update_map_data(new_data: Option<Location>, force_update: bool) {
    // Allow one task to wait on the update lock, turning away any others that
    // can't acquire the wait_lock immediately. This ensures there's always an
    // update that happens after map movement finishes.
    let app_state = AppState::global();
    let Ok(_wait_guard) = app_state.map_data.map_data_wait_lock.try_lock()
    else {
        return;
    };
    let _update_guard = app_state.map_data.map_data_update_lock.lock().await;
    drop(_wait_guard);

    let Some(map_state) = should_update_map_data(new_data, force_update).await
    else {
        return;
    };

    // The queries take the longest here, up to 500 ms.

    // point sizes can be large so it's worth expanding the viewport bounds
    // slightly
    let bounds = map_state.view_pos.expanded_bounds(BOUND_EXPANSION_PX);
    let make_points = map_state.style.marker_size > 0.0;
    let make_lines = map_state.style.line_size > 0;
    let params = DerivedParams {
        bounds,
        make_points,
        make_lines,
        lines_within_bounds: map_state.style.hide_points_outside_viewbounds,
    };

    let Some(mount_ids) =
        get_front_state(|s| s.mounted_db_settings.enabled_dbs())
    else {
        return;
    };
    let hard_cap = backend_backstop_points(mount_ids.len());
    let mut query_stats = common::state::MapQueryStats {
        hard_cap,
        ..Default::default()
    };
    // evict working sets of mounts that were disabled
    mvt::retain_tilesets(&mount_ids).await;

    let mut mount_data = BTreeMap::new();
    let mut global_cmap_params: Option<CmapParams> = None;
    for id in mount_ids {
        let Some(conn) = get_db_for_id(id) else {
            mount_data.insert(
                id,
                MountMapData {
                    params,
                    records: Vec::new(),
                    cmap_data: None,
                },
            );
            continue;
        };
        // TODO: parallelize
        let fetched =
            get_records_and_cmap_data(&conn, &map_state, &params, hard_cap)
                .await;
        // Lines need the time-adjacent points just outside the viewbounds.
        // fetch_adjacent finds where the track left the bounds via the id
        // stride of temporal decimation; bucketed ids have no stride, so
        // finding the neighbours would take a query per point. No lines.
        let mount_params = DerivedParams {
            make_lines: params.make_lines && !fetched.spatially_bucketed,
            ..params
        };
        query_stats.n_points += fetched.records.len();
        query_stats.duration_ms += fetched.query_time.as_millis() as u64;
        query_stats.decimated |= fetched.decimated;
        query_stats.memory_capped |= fetched.memory_capped;
        match (global_cmap_params, &fetched.cmap_data) {
            (Some(global), Some((new_params, _))) => {
                global_cmap_params = Some(global.merge(new_params))
            }
            (None, Some((new_params, _))) => {
                global_cmap_params = Some(*new_params)
            }
            _ => (),
        }
        mount_data.insert(
            id,
            MountMapData {
                params: mount_params,
                records: fetched.records,
                cmap_data: fetched.cmap_data,
            },
        );
    }
    tracing::debug!(
        "map query: {} points in {} ms (decimated: {})",
        query_stats.n_points,
        query_stats.duration_ms,
        query_stats.decimated,
    );
    crate::app_state::set_back_state(|s| s.last_map_query = Some(query_stats));
    if let Some(mut cmap_params) = global_cmap_params {
        // Trim the colormap to hold contrast against the basemap. The trim is
        // baked into the tile colors below; if the requery on light<->dark
        // basemap switches ever feels slow, store the normalized value in the
        // tileset instead and colorize at slice time in mvt.rs.
        cmap_params.trim = map_state.style.cmap_trim();
        // update the cmap parameters
        let mut persistent_guard = app_state.persistent.lock().unwrap();
        persistent_guard.back.cmap_params = cmap_params;
        // update parameters in each database set
        for data in mount_data.values_mut() {
            if let Some((cparams, _)) = data.cmap_data.as_mut() {
                *cparams = cmap_params;
            }
        }
    }
    // sends the query stats, and the cmap params if they updated
    ws_session::send_back_state_to_front();
    if get_front_state(|s| s.map.settings_tab.clone())
        == Some(common::state::MapSettingsTab::TimeSeriesPlot)
    {
        update_timeseries_plot_data(
            &mount_data,
            &map_state.style.colored_datastream,
        );
    }
    for (id, data) in mount_data.into_iter() {
        // TODO: parallelize?
        let (n_segs, tileset_bytes) =
            make_tileset(id, &data.params, data.records, data.cmap_data).await;
        query_stats.n_segs += n_segs;
        query_stats.tileset_bytes += tileset_bytes as u64;
    }
    // second send: update the stats with the tileset sizes
    query_stats.footprint_bytes = process_footprint_bytes();
    set_back_state(|s| s.last_map_query = Some(query_stats));
    ws_session::send_back_state_to_front();
    ws_session::send_message_to_front(ToFront::MapDataUpdated);
}

/// Parameters derived from map state which are used in multiple places.
#[derive(Clone, Copy)]
pub struct DerivedParams {
    bounds: LngLatBounds,
    make_points: bool,
    make_lines: bool,
    /// Segments need both endpoints in bounds rather than one, so nothing is
    /// drawn to off-screen points (hide_points_outside_viewbounds). Adjacent
    /// points are still fetched so segments join true neighbours.
    lines_within_bounds: bool,
}

impl DerivedParams {
    /// Whether the segment between consecutive records a and b is drawn.
    fn seg_visible(&self, a: &NarrowPoint, b: &NarrowPoint) -> bool {
        let (a, b) = (
            self.bounds.contains(&a.lnglat()),
            self.bounds.contains(&b.lnglat()),
        );
        if self.lines_within_bounds {
            a && b
        } else {
            a || b
        }
    }
}

/// Cell size for spatial decimation of the map's points, or None if the map
/// state calls for time-uniform decimation instead.
pub fn spatial_cell_for(map_state: &MapState) -> Option<database::SpatialCell> {
    let style = &map_state.style;
    // Dwell colorings are computed from time-adjacent points, the time series
    // plot needs a time-uniform sample, and a lines-only rendering (no
    // markers) wants a time-contiguous track.
    if style.decimation_mode != common::map_style::DecimationMode::Spatial
        || style.marker_size == 0.0
        || style.colored_datastream.uses_adjacent_points()
        || map_state.settings_tab
            == common::state::MapSettingsTab::TimeSeriesPlot
    {
        return None;
    }
    Some(database::SpatialCell::from_zoom(
        map_state.view_pos.zoom,
        style.spatial_cell_pitch.pixels(),
    ))
}

/// One mount's fetched records, held between the query pass and the
/// tileset build, with the params its tileset is built with (make_lines
/// may differ per mount from the global DerivedParams).
pub struct MountMapData {
    pub params: DerivedParams,
    pub records: Vec<NarrowPoint>,
    pub cmap_data: crate::metrics::dashboard::CmapData,
}

/// Records fetched for one mounted database, with diagnostics.
struct FetchedRecords {
    records: Vec<NarrowPoint>,
    cmap_data: Option<(CmapParams, Vec<Option<f64>>)>,
    decimated: bool,
    /// Whether the records were spatially bucketed (in which case lines
    /// shouldn't be drawn between them).
    spatially_bucketed: bool,
    /// Whether the hard_cap memory backstop bound the result.
    memory_capped: bool,
    query_time: std::time::Duration,
}

async fn get_records_and_cmap_data(
    conn: &SqlitePool,
    map_state: &MapState,
    params: &DerivedParams,
    hard_cap: u64,
) -> FetchedRecords {
    let DerivedParams {
        bounds,
        make_points: _,
        make_lines,
        lines_within_bounds: _,
    } = *params;
    let colored_datastream = &map_state.style.colored_datastream;

    // only bother with the performance overhead of getting points adjacent
    // to the viewbounds if lines are actually drawn
    // OR if the colormapping relies on the existence of out-of-bounds points to
    // improve colormapping (as is the case for dwells)
    let get_adjacent = make_lines || colored_datastream.should_get_adjacent();

    let before = std::time::Instant::now();
    let database::DecimatedResult {
        records,
        decimated,
        spatially_bucketed,
        memory_capped,
    } = database::FilteredQuery::builder()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(bounds)
        .get_adjacent(get_adjacent)
        .decimation_threshold(decimation_threshold(&map_state.style))
        .hard_cap(hard_cap)
        .maybe_spatial_cell(spatial_cell_for(map_state))
        .maybe_field1(field1_column(colored_datastream))
        .build()
        .fetch_decimated_result_with_db(conn)
        .await;
    let query_time = before.elapsed();

    // indicate which points are not visible and do not create a line segment
    // that will be visible when calculating the colormap
    let mut cmap_records = vec![];
    for i in 0..records.len() {
        let should_keep = bounds.contains(&records[i].lnglat())
            || (i < records.len() - 1
                && params.seg_visible(&records[i], &records[i + 1]));
        cmap_records.push((&records[i], should_keep));
    }
    let cmap_data = get_cmap_data(colored_datastream, &cmap_records);

    FetchedRecords {
        records,
        cmap_data,
        decimated,
        spatially_bucketed,
        memory_capped,
        query_time,
    }
}

/// Build and store the mount's vector-tile working set (mvt.rs). Returns
/// (n_segs, approx_bytes) of the stored tileset.
async fn make_tileset(
    mount_id: MountID,
    params: &DerivedParams,
    records: Vec<NarrowPoint>,
    cmap_data: Option<(CmapParams, Vec<Option<f64>>)>,
) -> (usize, usize) {
    let DerivedParams {
        bounds,
        make_points,
        make_lines,
        lines_within_bounds: _,
    } = *params;

    let mut tileset = mvt::TileSet::default();
    for i in 0..records.len() {
        // Only make a point if it's within our drawing boundary
        let make_point = make_points && bounds.contains(&records[i].lnglat());
        // With the lines we index one ahead to get the line endpoint, so we
        // don't want to make the line on the final record.
        let make_line = make_lines
            && i < records.len() - 1
            && params.seg_visible(&records[i], &records[i + 1]);
        if !make_point && !make_line {
            continue;
        }
        let color = cmap_data.as_ref().map(|(cmap_params, cmap_vals)| {
            cmap_vals[i]
                .map(|v| cmaps::get_data_color(v, cmap_params))
                .unwrap_or("#808080") // if data not known, use grey color
        });
        // z0 web-mercator coordinates
        let coord1 = records[i].lnglat();
        let t = TileXYZ::from_lnglat(&coord1, 0);
        if make_point {
            tileset.points.push(mvt::TilePoint {
                x: t.x,
                y: t.y,
                color,
            });
        }
        if make_line {
            let seg = |a: &TileXYZ, b: &TileXYZ| mvt::TileSeg {
                x1: a.x,
                y1: a.y,
                x2: b.x,
                y2: b.y,
                color,
            };
            let coord2 = records[i + 1].lnglat();
            let t2 = TileXYZ::from_lnglat(&coord2, 0);
            if crosses_antimeridian(&coord1, &coord2) {
                // cut so neither segment's bbox spans the world
                let (c1a, c2a) = antimeridian_cut(&coord1, &coord2);
                let t1a = TileXYZ::from_lnglat(&c1a, 0);
                let t2a = TileXYZ::from_lnglat(&c2a, 0);
                tileset.segs.push(seg(&t, &t1a));
                tileset.segs.push(seg(&t2a, &t2));
            } else {
                tileset.segs.push(seg(&t, &t2));
            }
        }
    }
    let n_segs = tileset.segs.len();
    let tileset_bytes = tileset.approx_bytes();
    mvt::store_tileset(mount_id, tileset).await;
    (n_segs, tileset_bytes)
}

/// Center and zoom level for the `zoom all data` button, fitting the data
/// extent under the current time range and filters. Queried on button press
/// (Request::DataViewParams): the MIN/MAX bounds query has no covering index
/// and scans every row in range, so it's too costly to run on every map
/// update.
pub async fn get_data_view_params() -> Option<(LngLat, f64)> {
    let (time_range, filters) = {
        let app_state = AppState::global();
        let persistent_guard = app_state.persistent.lock().unwrap();
        let map_state = &persistent_guard.front.as_ref()?.map;
        (map_state.time_range, map_state.filters.clone())
    };
    let before = std::time::Instant::now();
    let data_bounds = database::FilteredQuery::builder()
        .time_range(time_range)
        .filters(filters)
        .build()
        .fetch_bounds()
        .await;
    tracing::debug!(
        "zoom-all fetch_bounds: {} ms",
        before.elapsed().as_millis()
    );
    get_view_params(&data_bounds)
}

/// Returns true if the line between two coordinates crosses the antimeridian.
fn crosses_antimeridian(coord1: &LngLat, coord2: &LngLat) -> bool {
    (coord1.lng - coord2.lng).abs() > 180.0
}

/// Calculates the coordinates on the antimeridian (A.M.) where the line
/// between the endpoints meets it on each side.
///
/// Calculation is done in tile coordinates so that the midpoint calculated
/// results in a line that appears straight on the web mercator projection. Just
/// using lnglat coords would yield a bent line if away from the equator.
///
/// Method is a simple linear proportionality, going from coord1 to coord2,
/// where the change in y (latitude) at the antimeridian, dy, is:
///
///     dy = dx * delta_y / delta_x
///
/// - dx is the unsigned distance from coord1 to the antimeridian
/// - delta_y is the signed y distance between the coordinates
/// - delta_x is the unsigned distance between the coordinates across the A.M.
fn antimeridian_cut(coord1: &LngLat, coord2: &LngLat) -> (LngLat, LngLat) {
    // operate on zoom level 0 (single tile), where the lnglat bounds are 0-1.
    let z = 0;
    let t1 = TileXYZ::from_lnglat(coord1, z);
    let t2 = TileXYZ::from_lnglat(coord2, z);
    let x1 = t1.x;
    let x2 = t2.x;
    let y1 = t1.y;
    let y2 = t2.y;
    let delta_y = y2 - y1; // signed y distance
    let delta_x = 1. - (x1 - x2).abs(); // unsigned x distance
    let x1_sym = x1 - 0.5; // x1 symmetric around 0 (west is negative)
    let dx = 0.5 - x1_sym.abs(); // unsigned distance to the antimeridian (A.M.)
    let dy = dx * delta_y / delta_x; // signed y distance from coord1 on A.M.
    let new_y = y1 + dy;
    let signum01 = |val: f64| {
        // signum, scaled to the range 0-1 (0 for neg, 1 for pos)
        0.5 * (val.signum() + 1.)
    };
    let t1a = TileXYZ {
        x: signum01(x1_sym), // 0 if x1 is in the west, 1 if in east
        y: new_y,
        z,
    };
    let t2a = TileXYZ {
        x: signum01(-x1_sym), // 1 if x1 is in the west (x2 in east)
        y: new_y,
        z,
    };
    (t1a.to_lnglat(), t2a.to_lnglat())
}

/// Calculate the center of a map. Does not take the map size into account, so
/// is overly conservative (zooms further out than needed).
fn get_view_params(bounds: &Option<LngLatBounds>) -> Option<(LngLat, f64)> {
    bounds.as_ref().map(|b| {
        let lng_center = (b.ne.lng + b.sw.lng) / 2.;
        let lat_center = (b.ne.lat + b.sw.lat) / 2.;
        let lng_range = b.ne.lng - b.sw.lng;
        let lat_range = b.ne.lat - b.sw.lat;
        let lat_zoom =
            (360. / lat_range * lat_center.to_radians().cos()).log2();
        let lng_zoom = (360. / lng_range).log2();
        let zoom = if lat_zoom > lng_zoom {
            lng_zoom
        } else {
            lat_zoom
        };
        let mut zoom = zoom - 1.;
        zoom = zoom.clamp(0., 16.);
        (
            LngLat {
                lng: lng_center,
                lat: lat_center,
            },
            zoom,
        )
    })
}

/// Get the popup text for a click at a given location. Also takes the point's
/// color if it exists. Returns the location to put the popup, the text,
/// and the desired color of the popup's background
pub async fn get_location_near(
    mount_id: MountID,
    lnglat: LngLat,
) -> Option<ToFront> {
    let map_state = get_front_state(|front| front.map.clone())?;

    let Some(db) = get_db_for_id(mount_id) else {
        return None;
    };
    // Use the bound expansion and the backstop of the query that produced
    // the drawn points, so the decimation is identical and the point
    // clicked is among the records
    let hard_cap = get_back_state(|s| s.last_map_query)
        .map(|s| s.hard_cap)
        .unwrap_or_else(|| backend_backstop_points(1));
    let records = database::FilteredQuery::builder()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.expanded_bounds(BOUND_EXPANSION_PX))
        .decimation_threshold(decimation_threshold(&map_state.style))
        .hard_cap(hard_cap)
        .maybe_spatial_cell(spatial_cell_for(&map_state))
        .build()
        .fetch_decimated_with_db(&db)
        .await;

    let calc_dist = |loc: &NarrowPoint| {
        (loc.latitude - lnglat.lat).abs() + (loc.longitude - lnglat.lng).abs()
    };
    let mut min_distance = f64::INFINITY;
    let mut argmin = 0;
    for (i, loc) in records.iter().enumerate() {
        let dist = calc_dist(loc);
        // leq so if multiple points on the same spot, the last one is sent back
        // to the frontend, corresponding to the point that would be higher in
        // the map layers
        if dist <= min_distance {
            min_distance = dist;
            argmin = i;
        }
    }
    // the popup shows every field: hydrate the narrow point to the full
    // row via its UNIQUE timestamp
    let nearest =
        database::get_location_at(&db, records.get(argmin)?.timestamp).await?;
    let zdt = datetime_fn_infallible()(nearest.timestamp, nearest.lnglat());
    Some(ToFront::NearestLocation(mount_id, nearest, zdt))
}
