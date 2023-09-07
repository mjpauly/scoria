//! Manages the paths used to access files in the app.
//!
//! directory         | persistent? | sharable? | backed up? | may be purged?
//! ------------------|-------------|-----------|------------|---------------
//! Documents         |     y       |    y      |     y      |       _
//! Library           |     y       |    _      |     _      |       _
//! Library/AppSupp.  |     y       |    _      |     y      |       _
//! Library/Caches    |     _       |    _      |     _      |       y
//! tmp               |     _       |    _      |     _      |       y
//!
//! Docs: https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html
//!
//! CURRENT LAYOUT
//!
//! - Documents
//!     - database
//!     - stemlog.txt // no longer in use
//! - Library
//!     - persistent_state.json
//!             - ok to not back up, user can re-input their settings
//!             - also allows for different settings on different devices
//!     - unexplored_area
//!             - must be tied to persistent_state, since that tracks the last
//!               update time, also ok to re-derive on each device
//!     - logs
//! - tmp
//!     - map_cache
//!             - TODO: move to Library/Caches
//!
//! CHANGES
//!
//! - map_cache move from tmp -> Library/Caches, which is less greedily cleaned
//! - clean out stemlog, replace with log directory in Library/
//!

use std::path::PathBuf;

use crate::app_state::AppState;

const DB_PREFIX: &str = "sqlite://";
const DB_FNAME: &str = "data.db";

// name of the directory which contains cached map data
const MAP_CACHE_DIR: &str = "map_cache";
// name of the directory which contains the automap screen / unexplored area
const UNEXPLORED_AREA_DIR: &str = "unexplored_area";

// name of the directory containing hourly logs
const LOGS_DIR: &str = "logs";

// Struct that contains the app directory paths.
// Lets us keep the paths without having to pass it from Swift every function
// call.
#[allow(dead_code)] // not yet using some directories
#[derive(Debug, Clone)]
pub struct Paths {
    pub documents_dir: PathBuf,
    pub library_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub bundle_dir: PathBuf,
}

/// Get the documents directory in the app state, returning an ownable PathBuf.
pub fn get_documents_dir() -> PathBuf {
    AppState::global().paths.documents_dir.clone()
}

pub fn get_library_dir() -> PathBuf {
    AppState::global().paths.library_dir.clone()
}

pub fn get_bundle_dir() -> PathBuf {
    AppState::global().paths.bundle_dir.clone()
}

pub fn get_tmp_dir() -> PathBuf {
    AppState::global().paths.temp_dir.clone()
}

/// Resilient to being cleared out, so ok to use tmp directory
pub fn get_map_cache_dir() -> PathBuf {
    get_tmp_dir().join(MAP_CACHE_DIR)
}

/// Needs to be persistent, so we used the Library dir.
pub fn get_unexplored_data_dir() -> PathBuf {
    get_library_dir().join(UNEXPLORED_AREA_DIR)
}

/// Get the database path as a string.
pub fn get_db_path() -> String {
    get_db_path_helper(&AppState::global().paths)
}

pub fn get_db_path_helper(paths: &Paths) -> String {
    let db_path = format!(
        "{}{}",
        DB_PREFIX,
        std::path::Path::new(&paths.documents_dir)
            .join(DB_FNAME)
            .display()
    );
    db_path
}

pub fn get_logs_dir() -> PathBuf {
    get_logs_dir_helper(&AppState::global().paths)
}

pub fn get_logs_dir_helper(paths: &Paths) -> PathBuf {
    paths.library_dir.join(LOGS_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::test_setup;

    #[tokio::test]
    async fn test_set_paths() {
        let dir = String::from("./test_set_paths/");
        test_setup(&dir).await;
        assert_eq!(
            get_documents_dir(),
            PathBuf::from("./test_set_paths/Documents")
        );
        let expected =
            format!("{}{}{}{}", DB_PREFIX, dir, "Documents/", DB_FNAME);
        assert_eq!(get_db_path(), expected);
    }
}
