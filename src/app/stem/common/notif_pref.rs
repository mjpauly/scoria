//! Settings related to notifications, such as whether to notify the user when
//! the app is closed and locations stop being logged.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub should_notify_on_stop: bool,
}
