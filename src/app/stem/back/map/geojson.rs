//! Builds the geojson data to plot in maplibre.
//!
//! These routes are protected both by the scope, and with the session cookie
//! that is set on initial connection. The `_: Identity` extractor marks the
//! routes as requiring authentication for access.
//!
//! To debug slow queries, use the ExplainQuery created by explain_decimate().
//! ```
//! // prints the query and the query plan
//! database::FilteredQuery::new()
//!     .time_range(map_state.time_range)
//!     .filters(map_state.filters.clone())
//!     .bounds(bounds)
//!     .get_adjacent(make_lines)
//!     .explain_decimate(DECIMATION_THRESHOLD)
//!     .explain()
//!     .await;
//! ```

use std::collections::BTreeMap;

use actix_identity::Identity;
use actix_web::{
    http::header::ContentType, routes, web, HttpResponse, Responder,
};
use anyhow::Context;
use common::cmaps::CmapParams;
use common::mounted::{EnabledDBs, MountID};
use common::view_position::LngLatBounds;
use geojson::{Feature, FeatureCollection, GeoJson, JsonObject, Value};
use sqlx::SqlitePool;

use crate::app_state::{get_front_state, set_back_state};
use crate::core::new_data_is_visible;
use crate::database::get_db_for_id;
use crate::logs::LogErrorAndContinue;
use crate::map::coords::TileXYZ;
use crate::metrics::color::get_cmap_data;
use crate::metrics::dashboard::update_timeseries_plot_data;
use crate::server::no_caching_directives;
use crate::tz::datetime_fn_infallible;
use crate::{app_state::AppState, database, ws_session};
use common::{
    cmaps,
    state::{MapState, PersistedRoute},
    LngLat, Location, ToFront,
};

// The maximum number of data points to put into the geojson. If greater, we
// decimate (select every nth) by a factor large enough to get under 10k points.
// 10k is a sweet spot for fairly low database query and map render times.
pub static DECIMATION_THRESHOLD: u64 = 10_000;

/// View window expansion factor for fetching from the database. This factor
/// expands the width and height by this value. 0.04 -> 4%.
/// We fetch data points outside the window since points can have some width and
/// it's nice to not have them suddenly-pop in only when their center is inside
/// the view window.
pub const BOUND_EXPANSION: f64 = 0.04;

/// Route for points and lines from a mounted database.
#[routes]
#[get("/{mount_id}/{points_lines}.geojson")]
#[get("/analyze/{mount_id}/{points_lines}.geojson")]
pub async fn geojson_route(
    _: Identity,
    path: web::Path<(MountID, String)>,
) -> impl Responder {
    let (mount_id, points_or_lines) = path.into_inner();
    let app_state = AppState::global();
    let geojsons = app_state.map_data.mount_geojsons.lock().await;
    let geojson_str = match (geojsons.get(&mount_id), points_or_lines.as_str())
    {
        (Some((points, _)), "points") => points.clone(),
        (Some((_, lines)), "lines") => lines.clone(),
        (None, _) | (Some((..)), _) => empty_geojson().to_string(),
    };
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JSON))
        .insert_header(no_caching_directives())
        .body(geojson_str)
}

pub fn empty_geojson() -> GeoJson {
    GeoJson::from(std::iter::empty::<Feature>().collect::<FeatureCollection>())
}

fn feature_collection_from_vec(v: Vec<Feature>) -> FeatureCollection {
    FeatureCollection {
        features: v,
        bbox: None,
        foreign_members: None,
    }
}

/// Determine if the map state/style is different in a way that means we should
/// update the geojson.
fn map_state_is_different(prev: &MapState, curr: &MapState) -> bool {
    prev.time_range != curr.time_range
        || prev.filters != curr.filters
        || prev.view_pos != curr.view_pos
        || prev.style.colored_datastream != curr.style.colored_datastream
        // If marker of line size were zero previously, the old geojson may not
        // have the data, so we should update. Going from visibile to not
        // visible doesn't require an update and looks smoother if updating the
        // size sliders rapidly, so we'll save compute when next updating
        || ((prev.style.marker_size == 0) && (curr.style.marker_size != 0))
        || ((prev.style.line_size == 0) && (curr.style.line_size != 0))
        || (prev.style.hide_points_outside_viewbounds != curr.style.hide_points_outside_viewbounds)
        || prev.timeline_config != curr.timeline_config
}

/// Determine if we should update the geojson data
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
async fn should_update_geojson(
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

/// Build both the points and lines geojson. Doesn't necessarily update; that is
/// determined by should_update_geojson().
///
/// This function is called whenever the front state changes, new location data
/// is logged while the frontend is active, or it's force updated.
///
/// 'new_data' indicates if this is triggered by new location data as opposed to
/// a change to the map's style
pub async fn update_geojson(new_data: Option<Location>, force_update: bool) {
    // Allow one task to wait on the update lock, turning away any others that
    // can't acquire the wait_lock immediately. This ensures there's always an
    // update that happens after map movement finishes.
    let app_state = AppState::global();
    let Ok(_wait_guard) = app_state.map_data.geojson_wait_lock.try_lock()
    else {
        return;
    };
    let _update_guard = app_state.map_data.geojson_update_lock.lock().await;
    drop(_wait_guard);

    let Some(map_state) = should_update_geojson(new_data, force_update).await
    else {
        return;
    };

    // The things that take the longest are the queries (this part, up to
    // 500ms), and stringifying the geojson, which is about 150ms for 10k pts.

    // point sizes can be large so it's worth expanding the viewport bounds
    // slightly
    let bounds = map_state.view_pos.bounds.expand(BOUND_EXPANSION);
    let make_points = map_state.style.marker_size > 0;
    let make_lines = map_state.style.line_size > 0;
    let params = DerivedParams {
        bounds,
        make_points,
        make_lines,
    };

    let Some(mount_ids) =
        get_front_state(|s| s.mounted_db_settings.enabled_dbs())
    else {
        return;
    };

    let mut records_and_cmap_data = BTreeMap::new();
    let mut global_cmap_params: Option<CmapParams> = None;
    for id in mount_ids {
        let Some(conn) = get_db_for_id(id) else {
            records_and_cmap_data.insert(id, (Vec::new(), None));
            continue;
        };
        // TODO: parallelize
        let (records, cmap_data) =
            get_records_and_cmap_data(&conn, &map_state, &params).await;
        match (global_cmap_params, &cmap_data) {
            (Some(global), Some((new_params, _))) => {
                global_cmap_params = Some(global.merge(new_params))
            }
            (None, Some((new_params, _))) => {
                global_cmap_params = Some(*new_params)
            }
            _ => (),
        }
        records_and_cmap_data.insert(id, (records, cmap_data));
    }
    if let Some(cmap_params) = global_cmap_params {
        // update the cmap parameters
        let mut persistent_guard = app_state.persistent.lock().unwrap();
        persistent_guard.back.cmap_params = cmap_params;
        ws_session::send_back_state_to_front();
        // update parameters in each database set
        for (_, maybe_cmap_data) in records_and_cmap_data.values_mut() {
            if let Some((cparams, _)) = maybe_cmap_data.as_mut() {
                *cparams = cmap_params;
            }
        }
    }
    if get_front_state(|s| s.map.settings_tab.clone())
        == Some(common::state::MapSettingsTab::TimeSeriesPlot)
    {
        update_timeseries_plot_data(
            &records_and_cmap_data,
            &map_state.style.colored_datastream,
        );
    }
    for (id, (records, cmap_data)) in records_and_cmap_data.into_iter() {
        // TODO: parallelize?
        make_geojson(id, &params, records, cmap_data).await;
    }
    ws_session::send_message_to_front(ToFront::GeojsonUpdated);
    update_zoom_all_data(&map_state);
}

/// Parameters derived from map state which are used in multiple places.
struct DerivedParams {
    bounds: LngLatBounds,
    make_points: bool,
    make_lines: bool,
}

async fn get_records_and_cmap_data(
    conn: &SqlitePool,
    map_state: &MapState,
    params: &DerivedParams,
) -> (Vec<Location>, Option<(CmapParams, Vec<Option<f64>>)>) {
    let DerivedParams {
        bounds,
        make_points: _,
        make_lines,
    } = *params;
    let colored_datastream = &map_state.style.colored_datastream;

    // only bother with the performance overhead of getting points adjacent
    // to the viewbounds if lines are actually drawn
    // OR if the colormapping relies on the existence of out-of-bounds points to
    // improve colormapping (as is the case for dwells)
    let get_adjacent = !map_state.style.hide_points_outside_viewbounds
        && (make_lines || colored_datastream.should_get_adjacent());

    // let before = std::time::Instant::now();
    let records = database::FilteredQuery::builder()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(bounds)
        .get_adjacent(get_adjacent)
        .limit(DECIMATION_THRESHOLD)
        .build()
        .fetch_decimated_with_db(conn)
        .await;
    // tracing::info!("Full query took {:.6?}", before.elapsed());

    // indicate which points are not visible and do not create a line segment
    // that will be visible when calculating the colormap
    let mut cmap_records = vec![];
    for i in 0..records.len() {
        let should_keep = bounds.contains(&records[i].lnglat())
            || (i < records.len() - 1
                && bounds.contains(&records[i + 1].lnglat()));
        cmap_records.push((&records[i], should_keep));
    }
    let cmap_data = get_cmap_data(colored_datastream, &cmap_records);

    (records, cmap_data)
}

async fn make_geojson(
    mount_id: MountID,
    params: &DerivedParams,
    records: Vec<Location>,
    cmap_data: Option<(CmapParams, Vec<Option<f64>>)>,
) {
    let DerivedParams {
        bounds,
        make_points,
        make_lines,
    } = *params;

    let mut points = Vec::new();
    let mut lines = Vec::new();
    for i in 0..records.len() {
        // Only make a point if it's within our drawing boundary
        let make_point = make_points && bounds.contains(&records[i].lnglat());
        // With the lines we index one ahead to get the line endpoint, so we
        // don't want to make the line on the final record. We also only draw
        // lines where one endpoint is within the drawing boundary.
        let make_line = make_lines
            && i < records.len() - 1
            && (bounds.contains(&records[i].lnglat())
                || bounds.contains(&records[i + 1].lnglat()));

        let properties = cmap_data.as_ref().map(|(cmap_params, cmap_vals)| {
            let color = cmap_vals[i]
                .map(|v| cmaps::get_data_color(v, cmap_params))
                .unwrap_or("#808080"); // if data not known, use grey color
            let mut props = JsonObject::new();
            props.insert("color".into(), color.into());
            props
        });

        let coord1 = records[i].lnglat();
        if make_point {
            add_point(&coord1, &properties, &mut points);
        }
        if make_line {
            let coord2 = records[i + 1].lnglat();
            if crosses_antimeridian(&coord1, &coord2) {
                cut_and_add_lines(&coord1, &coord2, &properties, &mut lines);
            } else {
                add_line(&coord1, &coord2, &properties, &mut lines);
            }
        }
    }
    let points_geojson = GeoJson::from(feature_collection_from_vec(points));
    let lines_geojson = GeoJson::from(feature_collection_from_vec(lines));
    // Stringifying the geojson takes a while, so we create two tasks to do it
    // concurrently. We also spawn a task to determine the all-data view pos.
    let points_task =
        tokio::spawn(update_geojson_string(mount_id, points_geojson, true));
    let lines_task =
        tokio::spawn(update_geojson_string(mount_id, lines_geojson, false));
    let (points_handle, lines_handle) = tokio::join!(points_task, lines_task);
    points_handle
        .context("points task join")
        .log_error_and_continue();
    lines_handle
        .context("lines task join")
        .log_error_and_continue();
}

async fn update_geojson_string(
    mount_id: MountID,
    geojson: GeoJson,
    points: bool,
) {
    let geojson = geojson.to_string();
    let app_state = AppState::global();
    let mut mount_geojsons = app_state.map_data.mount_geojsons.lock().await;
    match (mount_geojsons.get_mut(&mount_id), points) {
        (Some((points, _)), true) => *points = geojson,
        (Some((_, lines)), false) => *lines = geojson,
        (None, true) => {
            mount_geojsons
                .insert(mount_id, (geojson, empty_geojson().to_string()));
        }
        (None, false) => {
            mount_geojsons
                .insert(mount_id, (empty_geojson().to_string(), geojson));
        }
    }
}

/// Determine the center and zoom level for the `zoom all data` button and
/// update the frontend.
fn update_zoom_all_data(map_state: &MapState) {
    let time_range = map_state.time_range;
    let filters = map_state.filters.clone();
    tokio::spawn(async move {
        let data_bounds = database::FilteredQuery::builder()
            .time_range(time_range)
            .filters(filters)
            .build()
            .fetch_bounds()
            .await;
        let view_params = get_view_params(&data_bounds);
        set_back_state(|back| back.data_center = view_params);
        ws_session::send_back_state_to_front();
    });
}

fn add_point(
    coord: &LngLat,
    properties: &Option<JsonObject>,
    points: &mut Vec<Feature>,
) {
    let mut feat = Feature::from(Value::Point(coord.into()));
    feat.properties = properties.clone();
    points.push(feat);
}

fn add_line(
    coord1: &LngLat,
    coord2: &LngLat,
    properties: &Option<JsonObject>,
    lines: &mut Vec<Feature>,
) {
    let mut feat =
        Feature::from(Value::LineString(vec![coord1.into(), coord2.into()]));
    feat.properties = properties.clone();
    lines.push(feat);
}

/// Returns true if the line between two coordinates crosses the antimeridian.
fn crosses_antimeridian(coord1: &LngLat, coord2: &LngLat) -> bool {
    (coord1.lng - coord2.lng).abs() > 180.0
}

/// Calculates the coordinate on the antimeridian (A.M.) between the endpoints
/// of a line which crosses it, then pushes the new multilinestring onto the
/// vector of features.
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
fn cut_and_add_lines(
    coord1: &LngLat,
    coord2: &LngLat,
    properties: &Option<JsonObject>,
    lines: &mut Vec<Feature>,
) {
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
    let coord1a = t1a.to_lnglat();
    let coord2a = t2a.to_lnglat();
    let mut feat = Feature::from(Value::MultiLineString(vec![
        vec![coord1.into(), (&coord1a).into()],
        vec![(&coord2a).into(), coord2.into()],
    ]));
    feat.properties = properties.clone();
    lines.push(feat);
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
    // use the bound expansion so the decimation is identical
    let mut records = database::FilteredQuery::builder()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(BOUND_EXPANSION))
        .limit(DECIMATION_THRESHOLD)
        .build()
        .fetch_decimated_with_db(&db)
        .await;

    let calc_dist = |loc: &common::Location| {
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
    let nearest = records.remove(argmin);
    let zdt = datetime_fn_infallible()(&nearest);
    Some(ToFront::NearestLocation(mount_id, nearest, zdt))
}
