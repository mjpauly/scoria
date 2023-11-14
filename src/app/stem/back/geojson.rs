//! Builds the geojson data to plot in maplibre.

use actix_web::{http::header::ContentType, routes, HttpResponse, Responder};
use geojson::{Feature, FeatureCollection, GeoJson, JsonObject, Value};

use crate::map::coords::TileXYZ;
use crate::server::no_caching_directives;
use crate::{app_state::AppState, database, ws_session};
use common::{
    cmaps, float,
    state::{MapState, PersistedRoute},
    units::UnitPreference,
    LngLat, Location, ToFront,
};

// The maximum number of data points to put into the geojson. If greater, we
// decimate (select every nth) by a factor large enough to get under 40k points.
pub static DECIMATION_THRESHOLD: u64 = 20_000;

#[routes]
#[get("/points.geojson")]
#[get("/analyze/points.geojson")]
pub async fn points_geojson_route() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JSON))
        .insert_header(no_caching_directives())
        .body(
            AppState::global()
                .map_data
                .lock()
                .unwrap()
                .points_geojson
                .to_string(),
        )
}

#[routes]
#[get("/lines.geojson")]
#[get("/analyze/lines.geojson")]
pub async fn lines_geojson_route() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType(mime::APPLICATION_JSON))
        .insert_header(no_caching_directives())
        .body(
            AppState::global()
                .map_data
                .lock()
                .unwrap()
                .lines_geojson
                .to_string(),
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
fn should_update_geojson(
    new_data: Option<Location>,
    foregrounded: bool,
) -> Option<MapState> {
    // get the map configuration state
    let app_state = AppState::global();
    let persistent_guard = app_state.persistent.lock().unwrap();
    // the '?' operator returns None if the frontend hasn't been initialized yet
    let map_state = &persistent_guard.front.as_ref()?.map;
    let mut map_data_guard = app_state.map_data.lock().unwrap();
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
        if let Some(prev_map_state) = &map_data_guard.prev_map_state {
            let new_data_in_time_range = new_data
                .map(|l| map_state.time_range.contains(&l.timestamp))
                .unwrap_or(false);
            if !new_data_in_time_range
                && !map_state_is_different(prev_map_state, map_state)
            {
                // same map state and no new data, don't bother updating
                return None;
            }
        }
    }
    // -> Should update if we get here <-
    // store the current state as the previous state
    map_data_guard.prev_map_state = Some(map_state.clone());
    Some(map_state.clone())
}

// TODO: for showing points only within a window, maybe add some margin to
// the edges so lines continue off the edge as expected? doesn't fix
// potential cross-screen wrapping, but we can try to drop those lines when
// making the geojson if they're really bad. kind of the problem of big line
// jumps more generally.

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
    let Some(map_state) = should_update_geojson(new_data, foregrounded) else {
        return
    };
    let colored_datastream = &map_state.style.colored_datastream;

    let records = database::FilteredQuery::new()
        .start(map_state.time_range.start)
        .end(map_state.time_range.end)
        .filters(map_state.filters.clone())
        .decimate(DECIMATION_THRESHOLD)
        .fetch_all()
        .await;

    let make_points = map_state.style.marker_size > 0;
    let make_lines = map_state.style.line_size > 0;
    let offset = map_state.time_range.start.offset();
    let cmap_params = colored_datastream.get_cmap_params(&records, &offset);

    let app_state = AppState::global();
    let mut persistent_guard = app_state.persistent.lock().unwrap();
    if records.is_empty() {
        persistent_guard.back.data_center = None;
    } else {
        persistent_guard.back.data_center = Some(get_view_params(&records));
    }
    persistent_guard.back.cmap_params = cmap_params.clone();
    drop(persistent_guard);
    // Send the new back state (cmap params and data center) to the frontend
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    if let Some(addr) = maybe_addr {
        addr.do_send(ws_session::SendState);
    }

    let mut points = Vec::new();
    let mut lines = Vec::new();
    for i in 0..records.len() {
        // With the lines we index one ahead to get the line endpoint, so we
        // don't want to make the line on the final record
        let make_line = make_lines && i < records.len() - 1;

        let properties = if colored_datastream.is_some() {
            let val = colored_datastream.get_stream(&records[i], &offset);
            let color = if let Some(known_val) = val {
                cmaps::get_data_color(known_val, &cmap_params)
            } else {
                // if data in this dimension not known, use a neutral gray color
                "#808080"
            };
            let mut props = JsonObject::new();
            props.insert("color".to_string(), color.into());
            Some(props)
        } else {
            None
        };

        let coord1 = records[i].lnglat();
        if make_points {
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
    let mut map_data_guard = app_state.map_data.lock().unwrap();
    map_data_guard.points_geojson = points_geojson;
    map_data_guard.lines_geojson = lines_geojson;

    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    if let Some(addr) = maybe_addr {
        addr.do_send(ws_session::MsgToFront(ToFront::GeojsonUpdated));
    }
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
/// is overly conservative (zooms further out than needed)
fn get_view_params(records: &[common::Location]) -> (LngLat, f64) {
    if records.is_empty() {
        return Default::default();
    }
    let lats: Vec<_> = records.iter().map(|x| x.latitude).collect();
    let lons: Vec<_> = records.iter().map(|x| x.longitude).collect();
    let lat_center = (float::max(&lats) + float::min(&lats)) / 2.;
    let lon_center = (float::max(&lons) + float::min(&lons)) / 2.;
    let lat_range = float::max(&lats) - float::min(&lats);
    let lon_range = float::max(&lons) - float::min(&lons);
    let lat_zoom = (360. / lat_range * lat_center.to_radians().cos()).log2();
    let lon_zoom = (360. / lon_range).log2();
    let zoom = if lat_zoom > lon_zoom {
        lon_zoom
    } else {
        lat_zoom
    };
    let mut zoom = zoom - 1.;
    zoom = zoom.clamp(0., 16.);
    (
        LngLat {
            lng: lon_center,
            lat: lat_center,
        },
        zoom,
    )
}

/// Get the popup text for a click at a given location. Also takes the point's
/// color if it exists. Returns the location to put the popup, the text,
/// and the desired color of the popup's background
// TODO: ignore data points not in view
pub async fn get_popup_text(
    lnglat: LngLat,
    data_color: Option<String>,
) -> Option<ToFront> {
    let (map_state, unit_pref) = {
        let state = AppState::global();
        let persistent = state.persistent.lock().unwrap();
        let front = persistent.front.as_ref().unwrap();
        (front.map.clone(), front.unit_pref.clone())
    };

    let records = database::FilteredQuery::new()
        .start(map_state.time_range.start)
        .end(map_state.time_range.end)
        .filters(map_state.filters.clone())
        .decimate(DECIMATION_THRESHOLD)
        .fetch_all()
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
    let alt = alt
        .map(|alt| format!("{alt}<br>"))
        .unwrap_or_else(String::new);
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
