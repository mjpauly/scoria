//! State for mounted databases.
//!
//! TODO: make the main database a member of the mounted db list, with #0, and
//! impl default for the collection.
//!
//! front state:
//!     BTreeMap<MountID, MountedDB>
//!
//! derived:
//!     databases present on disk

use serde::{Deserialize, Serialize};

pub const MAIN_DB_MOUNT_ID: MountID = 0;
pub const MAIN_DB_NAME: &str = "Main";

pub type MountID = u32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MountedDB {
    pub name: String,  // locally-assigned name
    pub enabled: bool, // whether to show the data or not
}

pub trait EnabledDBs {
    fn enabled_dbs(&self) -> Vec<MountID>;
}

impl EnabledDBs for std::collections::BTreeMap<MountID, MountedDB> {
    fn enabled_dbs(&self) -> Vec<MountID> {
        self.iter()
            .filter_map(|(id, setting)| setting.enabled.then_some(*id))
            .collect()
    }
}
