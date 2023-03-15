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
    // Log the location in our database
    database::log_location(lat, lon, accuracy, speed, course, datetime_epoch)
        .await
        .unwrap();
    // We first want to get the address, NOT in the "if let" scrutinee, since
    // the lock will be held for the whole if-block, and we won't be able to
    // await
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    // If the UI is active, we'll send it the new location to display
    if let Some(addr) = maybe_addr {
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
        // We'll also send the number of data points that have been recorded
        // in the past hour
        let count = database::count_records_past_hour().await;
        addr.do_send(ws_session::MsgToFront(
            common::ToFront::LocationsPastHour(count),
        ));
    }
}
