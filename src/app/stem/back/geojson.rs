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

use actix_identity::Identity;
use actix_web::{http::header::ContentType, routes, HttpResponse, Responder};
use common::view_position::LngLatBounds;
use geojson::{Feature, FeatureCollection, GeoJson, JsonObject, Value};

use crate::app_state::{get_front_state, set_back_state};
use crate::core::new_data_is_visible;
use crate::map::coords::TileXYZ;
use crate::metrics::color::get_cmap_data;
use crate::server::no_caching_directives;
use crate::{app_state::AppState, database, ws_session};
use common::{
    cmaps,
    state::{MapState, PersistedRoute},
    units::UnitPreference,
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

#[routes]
#[get("/points.geojson")]
#[get("/analyze/points.geojson")]
pub async fn points_geojson_route(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JSON))
        .insert_header(no_caching_directives())
        .body(
            AppState::global()
                .map_data
                .points_geojson
                .lock()
                .await
                .clone(),
        )
}

#[routes]
#[get("/lines.geojson")]
#[get("/analyze/lines.geojson")]
pub async fn lines_geojson_route(_: Identity) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JSON))
        .insert_header(no_caching_directives())
        .body(
            AppState::global()
                .map_data
                .lines_geojson
                .lock()
                .await
                .clone(),
        )
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
}

/// Determine if we should update the geojson data
///
/// update conditions:
/// app was foregrounded
/// || (the route is on the analyze tab
///     && (there is new data in time range
///         || the map state/style is different from before))
async fn should_update_geojson(
    new_data: Option<Location>,
    foregrounded: bool,
) -> Option<MapState> {
    // get the map configuration state
    let app_state = AppState::global();
    let mut prev_map_data_guard =
        app_state.map_data.prev_map_state.lock().await;
    let persistent_guard = app_state.persistent.lock().unwrap();
    // the '?' operator returns None if the frontend hasn't been initialized yet
    let map_state = &persistent_guard.front.as_ref()?.map;
    // Always update when app is foregrounded since data may have come in while
    // we were in the background. We don't update the geojson in the background
    // since it's costly.
    // 'if' blocks test if we should NOT update (passed by returning None)
    if !foregrounded {
        if let Some(p) = persistent_guard.front.as_ref() {
            if p.route != PersistedRoute::Analyze {
                // not looking at the map, don't update data
                return None;
            }
        } else {
            // No frontend, shouldn't happen if foregrounded
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
/// is logged while the frontend is active, or the app is foregrounded.
///
/// 'new_data' indicates if this is triggered by new location data as opposed to
/// a change to the map's style
///
/// 'foregrounded' indicates if this is triggered when the app is foregrounded
pub async fn update_geojson(new_data: Option<Location>, foregrounded: bool) {
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

    let Some(map_state) = should_update_geojson(new_data, foregrounded).await
    else {
        return;
    };
    let colored_datastream = &map_state.style.colored_datastream;

    // The things that take the longest are the queries (this part, up to
    // 500ms), and stringifying the geojson, which is about 150ms for 10k pts.

    // point sizes can be large so it's worth expanding the viewport bounds
    // slightly
    let bounds = map_state.view_pos.bounds.expand(BOUND_EXPANSION);
    let make_points = map_state.style.marker_size > 0;
    let make_lines = map_state.style.line_size > 0;

    // let before = std::time::Instant::now();
    let records = database::FilteredQuery::new()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(bounds)
        // only bother with the performance overhead of getting points adjacent
        // to the viewbounds if lines are actually drawn
        .get_adjacent(make_lines)
        .limit(DECIMATION_THRESHOLD)
        .fetch_decimated()
        .await;
    // tracing::info!("Full query took {:.6?}", before.elapsed());

    // filter out points that are not visible and do not create a line segment
    // that will be visible when calculating the colormap
    let mut cmap_records = vec![];
    for i in 0..records.len() {
        let should_keep = bounds.contains(&records[i].lnglat())
            || (i < records.len() - 1
                && bounds.contains(&records[i + 1].lnglat()));
        cmap_records.push((&records[i], should_keep));
    }
    let offset = map_state.time_range.start.offset();
    let cmap_data = get_cmap_data(colored_datastream, &cmap_records, &offset);

    if let Some((cmap_params, _)) = cmap_data {
        // update the cmap parameters
        let mut persistent_guard = app_state.persistent.lock().unwrap();
        persistent_guard.back.cmap_params = cmap_params;
        ws_session::send_back_state_to_front();
    }

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
    let points_task = tokio::spawn(update_points_geojson(points_geojson));
    let lines_task = tokio::spawn(update_lines_geojson(lines_geojson));
    update_zoom_all_data(&map_state);
    let (points_handle, lines_handle) = tokio::join!(points_task, lines_task);
    if let Err(e) = points_handle {
        tracing::error!("Points task join failure: {e}");
    }
    if let Err(e) = lines_handle {
        tracing::error!("Lines task join failure: {e}");
    }
    ws_session::send_message_to_front(ToFront::GeojsonUpdated);
}

async fn update_points_geojson(points_geojson: GeoJson) {
    let points_geojson = points_geojson.to_string();
    let app_state = AppState::global();
    *app_state.map_data.points_geojson.lock().await = points_geojson;
}

async fn update_lines_geojson(lines_geojson: GeoJson) {
    let lines_geojson = lines_geojson.to_string();
    let app_state = AppState::global();
    *app_state.map_data.lines_geojson.lock().await = lines_geojson;
}

/// Determine the center and zoom level for the `zoom all data` button and
/// update the frontend.
fn update_zoom_all_data(map_state: &MapState) {
    let time_range = map_state.time_range;
    let filters = map_state.filters.clone();
    tokio::spawn(async move {
        let data_bounds = database::FilteredQuery::new()
            .time_range(time_range)
            .filters(filters)
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
pub async fn get_popup_text(
    lnglat: LngLat,
    data_color: Option<String>,
) -> Option<ToFront> {
    let (map_state, unit_pref) =
        get_front_state(|front| (front.map.clone(), front.unit_pref))?;

    // use the bound expansion so the decimation is identical
    let records = database::FilteredQuery::new()
        .time_range(map_state.time_range)
        .filters(map_state.filters.clone())
        .bounds(map_state.view_pos.bounds.expand(BOUND_EXPANSION))
        .limit(DECIMATION_THRESHOLD)
        .fetch_decimated()
        .await;

    let local_offset = map_state.time_range.start.offset();

    let calc_dist = |loc: &common::Location| {
        (loc.latitude - lnglat.lat).abs() + (loc.longitude - lnglat.lng).abs()
    };
    let mut min_distance = f64::INFINITY;
    let mut argmin = 0;
    for (i, loc) in records.iter().enumerate() {
        let dist = calc_dist(loc);
        if dist < min_distance {
            min_distance = dist;
            argmin = i;
        }
    }
    // get() safely indexes into records so we return None if no data
    records.get(argmin).map(|loc| {
        let text = location_popup_text(local_offset, &unit_pref, loc);
        let bg_color = data_color
            .unwrap_or_else(|| map_state.style.solid_color.rgb.clone());
        ToFront::PopupText {
            location: LngLat {
                lng: loc.longitude,
                lat: loc.latitude,
            },
            text,
            bg_color,
        }
    })
}

pub fn location_popup_text(
    local_offset: time::UtcOffset,
    unit_pref: &UnitPreference,
    loc: &common::Location,
) -> String {
    let latlon = format!(
        "{}, {}",
        unit_pref.format_angle(loc.latitude, Some(6)),
        unit_pref.format_angle(loc.longitude, Some(6))
    );
    let mut accuracy_speed_course = format!(
        "±{}",
        unit_pref.format_small_length(loc.horizontal_accuracy, Some(2)),
    );

    if let Some(speed) = loc.speed {
        accuracy_speed_course +=
            &format!(", {}", unit_pref.format_velocity(speed, Some(2)));
    }
    if let Some(course) = loc.course {
        accuracy_speed_course +=
            &format!(", {}", unit_pref.format_angle(course, Some(2)));
    }

    let mut alt = loc.msl_altitude.map(|alt| {
        format!("{} altitude", unit_pref.format_small_length(alt, Some(2)))
    });
    if let Some(v_acc) = loc.vertical_accuracy {
        alt = alt.map(|alt| {
            format!("{alt} ±{}", unit_pref.format_small_length(v_acc, Some(2)))
        });
    }
    if let Some(story) = loc.story {
        alt = alt.map(|alt| format!("{alt}, story {story}"));
    }
    let alt = alt.map(|alt| format!("{alt}<br>")).unwrap_or_default();
    format!(
        "{}<br>\
        {}<br>\
        {}\
        {}",
        latlon,
        accuracy_speed_course,
        alt,
        loc.timestamp
            .to_offset(local_offset)
            .format(&time::format_description::well_known::Rfc2822)
            .unwrap_or_else(|_| "Timestamp unavailable".to_string())
    )
}
