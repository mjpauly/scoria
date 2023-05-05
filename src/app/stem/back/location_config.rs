//! Manages the Auto mode of the location config and figures out what to tell
//! the OS.
//!
//! Currentl Auto mode just does the Standard mode, but changes between Best
//! and 100 m accuracy depending on the number of data points that have been
//! collected in the past minute.
//!
//! If we're in a high accuracy mode and there's little movement, we switch to
//! a low accuracy mode. If we're in a low accuracy mode and there's movement,
//! we switch to a high accuracy mode.
//!
//! let record_count be the number of location records logged in the past minute
//! High -> Low accuracy happens if record_count < THRESHOLD
//! Low -> High accuracy happens if record_count >= THRESHOLD + HYSTERESIS
//!
//! The slowest walking pace is about 1m/s. We halve that for a ~0.5 m/s
//! threshold. With a distance_filter of 5.0 m,
//!     THRESHOLD = (0.5 m/s) * (60 s) / (5.0 m/count) = 6 counts
//! In other words, it takes 10s to move 5 m at a 0.5 m/s pace, which can happen
//! 6 times in a minute.
//!
//! Since a change in accuracy can trigger a new location update, we set the
//! hysteresis to 1 record count

use crate::app_state::AppState;
use crate::core::print_and_log;
use crate::database;
use crate::ws_session::MsgToFront;
use common::{AutoConfig, LocationAccuracyMode, LocationConfig, ToFront};

static THRESHOLD: i32 = 6;
static HYSTERESIS: i32 = 1;

/// Return whether the standard location service should be enabled
pub async fn get_standard_location_enabled() -> bool {
    update_auto_location_config().await;
    get_location_config().os_standard_on()
}

/// Return whether we should enabled the significant location changes service
pub fn get_significant_changes_enabled() -> bool {
    get_location_config().os_infrequent_on()
}

/// Return the distance filter setting (used for standard location service)
pub fn get_distance_filter() -> f32 {
    get_location_config().os_distance_filter()
}

/// Return the location accuracy mode (used for standard location service)
pub extern "C" fn get_location_accuracy_mode() -> LocationAccuracyMode {
    get_location_config().os_accuracy_mode()
}

/// Update the auto mode. Move from low accuracy to high accuracy when the user
/// starts moving, and the reverse when they're stationary.
async fn update_auto_location_config() {
    let config = get_location_config();
    if !config.auto_on() {
        // Not in auto mode, don't bother updating
        return;
    }
    if config.auto_standard_on()
        && config.auto_config.standard_config.accuracy_mode
            == LocationAccuracyMode::Best
    {
        // High accuracy mode -> see if we're stopped and should switch to a low
        // accuracy mode
        // TODO: if battery < 20% -> go to significant changes mode
        let minute_ago =
            time::OffsetDateTime::now_utc() - time::Duration::minutes(1);
        let count = database::count_records_since(minute_ago).await;
        if count < THRESHOLD {
            print_and_log(&format!(
                "switching to low accuracy, count past minute = {}",
                count
            ));
            set_auto_accuracy(LocationAccuracyMode::HundredMeters);
        }
    } else if config.auto_standard_on()
        && config.auto_config.standard_config.accuracy_mode
            == LocationAccuracyMode::HundredMeters
    {
        // Lower accuracy mode -> maybe go to higher accuracy
        let minute_ago =
            time::OffsetDateTime::now_utc() - time::Duration::minutes(1);
        let count = database::count_records_since(minute_ago).await;
        if count >= THRESHOLD + HYSTERESIS {
            print_and_log(&format!(
                "switching to high accuracy, count past minute = {}",
                count
            ));
            set_auto_accuracy(LocationAccuracyMode::Best);
        }
    } else {
        // shouldn't get here, but if we do we'll set the location mode to
        // the default auto config
        AppState::global()
            .persistent
            .lock()
            .unwrap()
            .location_config
            .auto_config = AutoConfig::default()
    }
}

/// Get a cloned copy of the current location config
fn get_location_config() -> LocationConfig {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .location_config
        .clone()
}

/// Set the accuracy mode of the standard mode config for auto
fn set_auto_accuracy(accuracy_mode: LocationAccuracyMode) {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .location_config
        .auto_config
        .standard_config
        .accuracy_mode = accuracy_mode;

    // persist the new state in case we are killed between location updates
    AppState::save_to_file();

    // update the UI in case it's open
    // (remember: no locks in `if let` scrutinee!)
    let maybe_addr = AppState::global().ws_addr.lock().unwrap().clone();
    if let Some(addr) = maybe_addr {
        let location_config = AppState::global()
            .persistent
            .lock()
            .unwrap()
            .location_config
            .clone();
        addr.do_send(MsgToFront(ToFront::LocationConfig(location_config)));
    }
}
