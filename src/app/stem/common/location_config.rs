//! Location configuration state
//!
//! User-facing state differs from the state that is told to the OS. Auto mode
//! is an option presented to the user, but under the hood the app just switches
//! between different accuracy levels of Standard mode depending on detected
//! movement (or infrequent mode if battery is low).
//!
//! Custom mode is our user-facing name for Standard mode.
//!
//! Infrequent mode is our name for the Significant Changes Service. Standard
//! mode is the shortened name of the Standard Location Service. Both of these
//! are actual location service modes in iOS.
//!
//! "OS" is used to refer to the mode that is actually told to the OS, as
//! opposed to the mode that the user sees.
//!
//! "Auto" and "Automatic" are used interchangeably.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Complete location configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone, Default)]
#[serde(default)]
pub struct AllLocationConfig {
    pub user: UserConfig,
    pub auto: AutoConfig,
}

impl AllLocationConfig {
    /// Returns whether auto mode is on and currently set to Standard
    pub fn auto_standard_on(&self) -> bool {
        self.user.auto_on() && self.auto.standard_on()
    }
    /// Returns whether auto mode is on and currently set to Infrequent
    pub fn auto_infrequent_on(&self) -> bool {
        self.user.auto_on() && self.auto.infrequent_on()
    }
    /// Returns whether the OS mode is set to Standard
    pub fn os_standard_on(&self) -> bool {
        self.user.standard_on()
            || self.auto_standard_on()
            || self.user.reduced_on()
    }
    /// Returns whether the OS mode is set to Infrequent
    pub fn os_infrequent_on(&self) -> bool {
        self.user.infrequent_on() || self.auto_infrequent_on()
    }
    /// Returns the distance filter that the OS should know
    pub fn os_distance_filter(&self) -> f32 {
        if self.user.auto_on() {
            self.auto.standard_config.distance_filter
        } else if self.user.reduced_on() {
            5.0
        } else {
            self.user.standard_config.distance_filter
        }
    }
    /// Returns the accuracy mode that the OS should know
    pub fn os_accuracy_mode(&self) -> LocationAccuracyMode {
        if self.user.auto_on() {
            self.auto.standard_config.accuracy_mode
        } else if self.user.reduced_on() {
            LocationAccuracyMode::HundredMeters
        } else {
            self.user.standard_config.accuracy_mode
        }
    }
}

/// Location configuration user settings
#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[serde(default)]
pub struct UserConfig {
    pub enabled: bool,
    pub mode: LocationMode, // user-facing location mode (includes "auto")
    pub standard_config: StandardLocationConfig, // standard mode user settings
}

impl UserConfig {
    pub fn is_auto(&self) -> bool {
        self.mode == LocationMode::Auto
    }
    pub fn is_reduced(&self) -> bool {
        self.mode == LocationMode::Reduced
    }
    pub fn is_standard(&self) -> bool {
        self.mode == LocationMode::Standard
    }
    pub fn is_infrequent(&self) -> bool {
        self.mode == LocationMode::SignificantChanges
    }
    pub fn auto_on(&self) -> bool {
        self.enabled && self.is_auto()
    }
    pub fn reduced_on(&self) -> bool {
        self.enabled && self.is_reduced()
    }
    pub fn standard_on(&self) -> bool {
        self.enabled && self.is_standard()
    }
    pub fn infrequent_on(&self) -> bool {
        self.enabled && self.is_infrequent()
    }
}

/// The location settigns we get at initial install, or if we fail to retrieve
/// the previous state when restarting.
impl Default for UserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: LocationMode::Auto,
            standard_config: StandardLocationConfig {
                accuracy_mode: LocationAccuracyMode::Best,
                distance_filter: 5.0,
            },
        }
    }
}

/// User-settable location modes. Differs from OSLocationMode
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum LocationMode {
    #[strum(serialize = "Automatic")]
    Auto, // Switches between Best and 100m accuracy
    #[strum(serialize = "Reduced")]
    Reduced, // Standard service at 100m accuracy always
    #[strum(serialize = "Infrequent")]
    SignificantChanges,
    #[strum(serialize = "Custom")]
    Standard,
}

/// Configuration of the Standard Mode (either user settings or auto mode)
#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub struct StandardLocationConfig {
    pub accuracy_mode: LocationAccuracyMode,
    pub distance_filter: f32,
}

/// Accuracy modes for the Standard Mode
/// NOTE: unsafety if out of sync with //src/app/stem/stem.h
/// TODO: use cxx.rs to generate safe bindings
#[repr(C)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
pub enum LocationAccuracyMode {
    #[strum(serialize = "Best")]
    Best,
    #[strum(serialize = "10 m")]
    TenMeters,
    #[strum(serialize = "100 m")]
    HundredMeters,
    #[strum(serialize = "1 km")]
    Kilometer,
    #[strum(serialize = "3 km")]
    ThreeKilometers,
}

/// State of the Auto config (what we tell the OS if Auto is on)
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AutoConfig {
    pub mode: OSLocationMode,
    pub standard_config: StandardLocationConfig,
}

impl AutoConfig {
    pub fn standard_on(&self) -> bool {
        self.mode == OSLocationMode::Standard
    }
    pub fn infrequent_on(&self) -> bool {
        self.mode == OSLocationMode::SignificantChanges
    }
}

impl Default for AutoConfig {
    fn default() -> Self {
        Self {
            mode: OSLocationMode::Standard,
            standard_config: StandardLocationConfig {
                accuracy_mode: LocationAccuracyMode::Best,
                distance_filter: 5.0,
            },
        }
    }
}

/// Possible location modes that can actually be set. UserConfig is
/// user-facing, while this is OS-facing
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum OSLocationMode {
    Standard,
    SignificantChanges,
}
