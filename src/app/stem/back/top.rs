//! High-level app logic that spans multiple modules. E.g. when we get new
//! location data, we want to 1) store it in the database, 2) send an update to
//! the UI, 3) do any additional calculations system visibility or user
//! analysis.

use crate::app_state::AppState;
use crate::common;
use crate::database;
use crate::ws_session;

pub async fn log_location(
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    datetime_epoch: i64,
) {
    database::log_location(lat, lon, accuracy, speed, course, datetime_epoch)
        .await
        .unwrap();
    if let Some(addr) = AppState::global().ws_addr.lock().unwrap().clone() {
        let datetime =
            time::OffsetDateTime::from_unix_timestamp(datetime_epoch).unwrap();
        let loc = common::Location {
            lat,
            lon,
            accuracy,
            speed,
            course,
            datetime,
        };
        addr.do_send(ws_session::MsgToFront(common::ToFront::LastLocation(
            loc,
        )));
    }
}
