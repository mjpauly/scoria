//! Synthetic-data probe mode for the dev binary, used by the memory probe
//! harness (doc/decimation/memory-limits.md, "Probe harness"). Bulk-generates
//! location data in a known viewport and seeds the persistent front state so
//! the frontend opens straight onto the analyze map with a chosen style.

use rand::random;
use stem::database::OSLocationData;

#[derive(Clone, Copy)]
enum Shape {
    Walk,     // track-like, d~1
    Scatter,  // uniform over the viewport, d~2
    Clusters, // tight clusters, d~0
}

#[derive(Clone, Copy)]
enum StyleConfig {
    Points, // markers only
    Lines,  // markers + lines
    Cmap,   // markers + lines + per-point color properties
}

/// Configuration for synthetic-data probe mode.
pub struct SynthConfig {
    count: u64,
    shape: Shape,
    style: StyleConfig,
    pub threshold: u64,
    view_zoom: f64,
    spatial: bool,
}

impl SynthConfig {
    /// Parse `--synth N [--shape walk|scatter|clusters]
    /// [--style points|lines|cmap] [--threshold T] [--view-zoom Z]
    /// [--decimation temporal|spatial]` (`--port` is parsed in main).
    /// Returns None if --synth is absent (normal dev mode).
    pub fn from_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let find = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .map(|i| args[i + 1].clone())
        };
        let count: u64 =
            find("--synth")?.parse().expect("--synth takes a count");
        let shape = match find("--shape").as_deref() {
            None | Some("walk") => Shape::Walk,
            Some("scatter") => Shape::Scatter,
            Some("clusters") => Shape::Clusters,
            Some(other) => panic!("unknown shape {other}"),
        };
        let style = match find("--style").as_deref() {
            None | Some("points") => StyleConfig::Points,
            Some("lines") => StyleConfig::Lines,
            Some("cmap") => StyleConfig::Cmap,
            Some(other) => panic!("unknown style {other}"),
        };
        let threshold = find("--threshold")
            .map(|t| t.parse().expect("threshold"))
            .unwrap_or_else(|| (count * 2).max(10_000));
        let view_zoom = find("--view-zoom")
            .map(|z| z.parse().expect("view-zoom"))
            .unwrap_or(DATA_ZOOM);
        let spatial = match find("--decimation").as_deref() {
            None | Some("temporal") => false,
            Some("spatial") => true,
            Some(other) => panic!("unknown decimation mode {other}"),
        };
        Some(Self {
            count,
            shape,
            style,
            threshold,
            view_zoom,
            spatial,
        })
    }
}

/// Center and viewport for probe runs (the headless browser is 1200x800).
const CENTER_LNG: f64 = -122.5;
const CENTER_LAT: f64 = 37.8;
/// Data always fills the zoom-12 viewport regardless of --view-zoom, so a
/// lower view zoom packs the same N into fewer, denser tiles (per-tile cost
/// probe; z10 puts everything in about one tile).
const DATA_ZOOM: f64 = 12.0;
const VIEW_W_PX: f64 = 1200.0;
const VIEW_H_PX: f64 = 800.0;

/// (lng span, lat span) of the viewport at the given zoom.
/// web mercator: world is 512 * 2^zoom px wide.
fn view_spans(zoom: f64) -> (f64, f64) {
    let world_px = 512.0 * 2f64.powf(zoom);
    let lng_span = 360.0 * VIEW_W_PX / world_px;
    let lat_span =
        lng_span * CENTER_LAT.to_radians().cos() * VIEW_H_PX / VIEW_W_PX;
    (lng_span, lat_span)
}

fn probe_view_pos(view_zoom: f64) -> common::view_position::ViewPosition {
    let (lng_span, lat_span) = view_spans(view_zoom);
    common::view_position::ViewPosition {
        center: common::LngLat {
            lng: CENTER_LNG,
            lat: CENTER_LAT,
        },
        zoom: view_zoom,
        bearing: 0.0,
        pitch: 0.0,
        bounds: common::view_position::LngLatBounds {
            sw: common::LngLat {
                lng: CENTER_LNG - lng_span / 2.0,
                lat: CENTER_LAT - lat_span / 2.0,
            },
            ne: common::LngLat {
                lng: CENTER_LNG + lng_span / 2.0,
                lat: CENTER_LAT + lat_span / 2.0,
            },
        },
    }
}

/// Bounding box synthetic data is generated in: the zoom-12 viewport minus a
/// margin, independent of --view-zoom. Returns (x0, y0, x1, y1) as
/// (lng, lat) pairs.
fn data_bbox() -> (f64, f64, f64, f64) {
    let (lng_span, lat_span) = view_spans(DATA_ZOOM);
    let sw = (CENTER_LNG - lng_span / 2.0, CENTER_LAT - lat_span / 2.0);
    (
        sw.0 + 0.1 * lng_span,
        sw.1 + 0.1 * lat_span,
        sw.0 + 0.9 * lng_span,
        sw.1 + 0.9 * lat_span,
    )
}

/// Seed the persistent front state so the frontend opens the analyze page
/// with the requested style, the probe viewport, and a time range covering
/// the synthetic data. Reaches the frontend in the ToFront::Startup message.
pub fn seed_front_state(cfg: &SynthConfig) {
    use common::map_style::{BasemapStyle, ColoredDataStream, DecimationMode};
    let mut state = common::FrontState {
        route: common::state::PersistedRoute::Analyze,
        last_viewed_intro_version: 999,
        ..Default::default()
    };
    state.mounted_db_settings.insert(
        common::mounted::MAIN_DB_MOUNT_ID,
        common::mounted::MountedDB {
            name: common::mounted::MAIN_DB_NAME.into(),
            enabled: true,
        },
    );
    let style = &mut state.map.style;
    style.basemap_style = BasemapStyle::None; // no network tile fetches
    style.marker_size = 4.0;
    style.line_size = match cfg.style {
        StyleConfig::Points => 0,
        _ => 2,
    };
    style.colored_datastream = match cfg.style {
        StyleConfig::Cmap => ColoredDataStream::Speed,
        _ => ColoredDataStream::None,
    };
    style.decimation_mode = if cfg.spatial {
        DecimationMode::Spatial
    } else {
        DecimationMode::Temporal
    };
    style.show_last_location = false;
    style.automap = false;
    let now = jiff::Timestamp::now().as_second();
    let ts = |s| jiff::Timestamp::from_second(s).unwrap();
    state.map.time_range = common::time_range::TimeRange {
        start: ts(now - 60 * 86_400),
        end: ts(now + 3_600),
    };
    state.map.time_delta_range = common::time_range::TimeDeltaRange {
        start_offset: -jiff::Span::new().days(60),
        end_offset: jiff::Span::new().hours(1),
        snap_start_to_day: false,
        snap_end_to_day: false,
    };
    state.map.view_pos = probe_view_pos(cfg.view_zoom);
    stem::app_state::AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front = Some(state);
}

/// Generate synthetic points and bulk insert them into the main database.
/// Timestamps are 1 Hz, ending one minute before now.
pub async fn insert_synth_data(cfg: &SynthConfig) {
    let n = cfg.count as usize;
    let (x0, y0, x1, y1) = data_bbox();
    let (w, h) = (x1 - x0, y1 - y0);
    let mut positions = Vec::with_capacity(n);
    match cfg.shape {
        Shape::Walk => {
            let (mut x, mut y) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            // step size that covers the box in about sqrt(n) steps
            let sx = 1.5 * w / (n as f64).sqrt();
            let sy = 1.5 * h / (n as f64).sqrt();
            let reflect = |v: f64, lo: f64, hi: f64| {
                if v < lo {
                    2.0 * lo - v
                } else if v > hi {
                    2.0 * hi - v
                } else {
                    v
                }
            };
            for _ in 0..n {
                x = reflect(x + (random::<f64>() - 0.5) * 2.0 * sx, x0, x1);
                y = reflect(y + (random::<f64>() - 0.5) * 2.0 * sy, y0, y1);
                positions.push((x, y));
            }
        }
        Shape::Scatter => {
            for _ in 0..n {
                positions
                    .push((x0 + random::<f64>() * w, y0 + random::<f64>() * h));
            }
        }
        Shape::Clusters => {
            let m = 20;
            let centers: Vec<(f64, f64)> = (0..m)
                .map(|_| (x0 + random::<f64>() * w, y0 + random::<f64>() * h))
                .collect();
            for _ in 0..n {
                let (cx, cy) = centers[(random::<f64>() * m as f64) as usize];
                positions.push((
                    cx + (random::<f64>() - 0.5) * w / 200.0,
                    cy + (random::<f64>() - 0.5) * h / 200.0,
                ));
            }
        }
    }
    let start_ts = jiff::Timestamp::now().as_second() - 60 - n as i64;
    // STEM_SYNTH_SPEED=cruise holds speed constant over 30-300 sample
    // segments (realistic same-color runs for cmap merge tests); default
    // is i.i.d. uniform, the worst case for run merging.
    let cruise = std::env::var("STEM_SYNTH_SPEED").as_deref() == Ok("cruise");
    let mut cruise_speed = 15.0;
    let mut cruise_left = 0usize;
    let mut locs = Vec::with_capacity(n);
    for (i, (lng, lat)) in positions.into_iter().enumerate() {
        locs.push(OSLocationData {
            timestamp: start_ts + i as i64,
            latitude: lat,
            longitude: lng,
            horizontal_accuracy: random::<f64>() * 3.0 + 2.0,
            msl_altitude: random::<f64>() * 100.0,
            ellipsoid_altitude: random::<f64>() * 100.0 + 30.0,
            vertical_accuracy: 3.0,
            story_available: true,
            story: i as i64,
            speed: if cruise {
                if cruise_left == 0 {
                    cruise_left = 30 + (random::<f64>() * 270.0) as usize;
                    cruise_speed = random::<f64>() * 30.0;
                }
                cruise_left -= 1;
                cruise_speed
            } else {
                random::<f64>() * 30.0
            },
            speed_accuracy: 1.0,
            course: random::<f64>() * 360.0,
            course_accuracy: 5.0,
            source_info_available: false,
            is_simulated_by_software: false,
            is_produced_by_accessory: false,
        });
    }
    // chunk to keep individual transactions reasonably sized
    for chunk in locs.chunks(50_000) {
        stem::database::log_locations_bulk(chunk.to_vec())
            .await
            .expect("bulk insert failed");
    }
    println!("inserted {n} synthetic points");
}
