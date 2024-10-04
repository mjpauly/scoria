//! Kalman Filtering to fuse IMU data with posistion
//!
//! Last location is used as the ENU origin, which all later positions are
//! converted to.

use common::{LngLat, Location};
use nalgebra::{Matrix3, Point3, Vector3};
use nav_types::{ENU, WGS84};
use time::OffsetDateTime;

use crate::app_state::{get_back_state, set_derived_state, AppState};

// const UPDATE_RATE_HZ: f64 = 50.0;

// based on test data captured on a stationary iPhone 12
// const ACCELERATION_VARIANCE: f32 = 1e-4; // (m/s^2) ^2
const ACCELERATION_VARIANCE: f32 = 1e-2; // (m/s^2) ^2

// const ROTATION_VARIANCE: f32 = 1e-5; // (rad/s) ^2
const ROTATION_VARIANCE: f32 = 1e-2; // (rad/s) ^2

// assume 1000m altitude standard deviation, and an altitude of 0, if the
// altitude is unknown.
const ALTITUDE_IF_UNKNOWN: f64 = 0.0;
const ALTITUDE_STD_IF_UNKNOWN: f64 = 1000.0;

/// FFI-compatible type for a 3-vector with XYZ
#[repr(C)]
#[derive(Clone, Debug)]
pub struct ThreeAxisData {
    x: f64,
    y: f64,
    z: f64,
}

impl From<ThreeAxisData> for Vector3<f32> {
    fn from(item: ThreeAxisData) -> Self {
        Vector3::new(item.x as f32, item.y as f32, item.z as f32)
    }
}

#[derive(Debug, Clone)]
pub struct State {
    origin: Option<WGS84<f64>>, // reference point for ENU origin
    filter: eskf::ESKF,
    last_imu_time: OffsetDateTime, // time of last prediction with IMU data
    last_position_time: OffsetDateTime, // last time position was injected
}

/// Use last location to set initial ENU reference frame, build filter with
/// estimated sensor variances.
///
/// Runs as a part of AppState initialization.
pub fn init(last_location: &Option<Location>) -> State {
    let filter = eskf::Builder::new()
        .acceleration_variance(ACCELERATION_VARIANCE)
        .rotation_variance(ROTATION_VARIANCE)
        .initial_covariance(1e-1)
        .build();
    let mut state = State {
        origin: None,
        filter,
        last_imu_time: now(),
        // - time::Duration::seconds_f64(1.0 / UPDATE_RATE_HZ),
        last_position_time: now(),
    };
    // update state using the last known location, if available
    if let Some(location) = last_location {
        // start with last location as first measurement
        observe_position(&mut state, location);
    }
    state
}

/// (temporary), log imu data to file for testing
// pub fn log_imu_data(acceleration: ThreeAxisData, rotation: ThreeAxisData) {
// }

pub fn update_with_position(location: &Location) {
    let binding = AppState::global();
    let mut state = binding.kf_state.lock().unwrap();
    observe_position(&mut state, location);
    update_solved_position(&state);
}

pub fn update_with_imu(acceleration: ThreeAxisData, rotation: ThreeAxisData) {
    let binding = AppState::global();
    let mut state = binding.kf_state.lock().unwrap();
    update_imu(&mut state, acceleration.into(), rotation.into());
    if let Some(location) = get_back_state(|s| s.last_location.clone()) {
        // if last location available, re-inject it every second
        if now() - state.last_position_time > time::Duration::seconds(1) {
            println!("position: {:?}", state.filter.position);
            println!("velocity: {:?}", state.filter.velocity);
            println!("orientation: {:?}", state.filter.orientation);
            println!(
                "roll pitch yaw: {:?}",
                state.filter.orientation.euler_angles()
            );
            println!("accel_bias: {:?}", state.filter.accel_bias);
            println!("rot_bias: {:?}", state.filter.rot_bias);
            println!("gravity: {:?}", state.filter.gravity);
            observe_position(&mut state, &location);
        }
    }
    update_solved_position(&state);
}

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

/// Update the filter using "ground-truth" position data.
fn observe_position(state: &mut State, location: &Location) {
    let horizontal_var = location.horizontal_accuracy.powi(2) as f32;
    let vertical_var = location
        .vertical_accuracy
        .unwrap_or(ALTITUDE_STD_IF_UNKNOWN)
        .powi(2) as f32;
    let variance = Matrix3::from_diagonal(&Vector3::new(
        horizontal_var, // east
        horizontal_var, // north
        vertical_var,   // up
    ));
    let wgs = location_to_wgs(location);
    // if no origin is set, use this observation as the origin
    let origin = *state.origin.get_or_insert(wgs);
    let enu = wgs - origin;
    if state
        .filter
        .observe_position(enu_to_point(enu), variance)
        .is_err()
    {
        tracing::error!("Filter inversion error.");
    }
    state.last_position_time = now();
}

/// Update the filter state using IMU data.
fn update_imu(
    state: &mut State,
    acceleration: Vector3<f32>,
    rotation: Vector3<f32>,
) {
    let now = now();
    let delta = now - state.last_imu_time;
    state.last_imu_time = now;
    state
        .filter
        .predict(acceleration, rotation, delta.unsigned_abs());
}

/// Update the LngLat position that is reported to the UI for display.
fn update_solved_position(state: &State) {
    let pos = state.filter.position;
    let enu = ENU::new(pos.x as f64, pos.y as f64, pos.z as f64);
    let solved = state.origin.map(|o| {
        let wgs = o + enu;
        LngLat {
            lng: wgs.longitude_degrees(),
            lat: wgs.latitude_degrees(),
        }
    });
    set_derived_state(|s| s.filtered_position = solved);
}

// TODO: handle case where position is far from origin, and f32 precision
// becomes a problem

fn location_to_wgs(l: &Location) -> WGS84<f64> {
    WGS84::from_degrees_and_meters(
        l.latitude,
        l.longitude,
        l.ellipsoid_altitude.unwrap_or(ALTITUDE_IF_UNKNOWN),
    )
}

fn enu_to_point(enu: ENU<f64>) -> Point3<f32> {
    Point3::new(enu.east() as f32, enu.north() as f32, enu.up() as f32)
}
