//! Core Scoria library interface.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.
//!
//! Our top-level lib.rs file contains the highest level of initialization in
//! these functions:
//!     set_app_dirs()
//!     init()
//!
//! Additional top-level functions are those which are exposed as the API to the
//! swift wrapper.
//!

pub mod app_state; // backend state storage
pub mod core; // high-level app logic that spans multiple modules
pub mod database; // manages the SQLite database
pub mod export; // export data to common geo data file formats
pub mod geojson; // construct the data to display in the frontend
pub mod location_config; // location logging configuration
pub mod logs;
pub mod map;
pub mod paths; // stores and retrieve file system paths
pub mod runtime; // retrieves async runtime for use in the sync C interface
pub mod server; // server for the UI
pub mod ws_session; // websocket actor for the UI

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

/// Set the directories known to the Rust library.
///
/// This should be called first thing when the app is launched.
///
/// The directories and their functions are:
///     - documents: stores persistent user data
///     - library: stores persistent application data
///     - temp: stores temporary data
///     - bundle: stores read-only data that is packaged with the app
#[no_mangle]
pub extern "C" fn set_app_dirs(
    documents_dir: *const c_char,
    library_dir: *const c_char,
    temp_dir: *const c_char,
    bundle_dir: *const c_char,
    app_version: *const c_char,
) {
    let paths_to_set = paths::Paths {
        documents_dir: PathBuf::from(cstr_to_string(documents_dir)),
        library_dir: PathBuf::from(cstr_to_string(library_dir)),
        temp_dir: PathBuf::from(cstr_to_string(temp_dir)),
        bundle_dir: PathBuf::from(cstr_to_string(bundle_dir)),
    };
    let app_version = cstr_to_string(app_version);
    runtime::get_runtime()
        .block_on(async { init(paths_to_set, app_version).await })
}

/// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Top app level initialization. Does not start the UI server yet; that is done
/// when the app enters the foreground and calls `handle_enter_foreground`.
pub async fn init(init_paths: paths::Paths, app_version: String) {
    // initialize the tracing infrastructure here, since it's universal to dev
    // and release (this belies how our stem "library" acts in like a binary)
    let subscriber = logs::get_subscriber(&init_paths);
    logs::init_logging(subscriber);
    let span = tracing::span!(tracing::Level::INFO, "init");
    let _enter = span.enter();

    let db = database::init_db(paths::get_db_path_helper(&init_paths))
        .await
        .unwrap();
    app_state::AppState::init(init_paths, app_version, db);
    // vacuum and checkpoint the database at startup, so it shrinks to size
    database::checkpoint_db().await;
    tracing::info!("===== App Startup =====");
}

/// Error logging and handling is handled with tracing, so we have the app
/// wrapper pass any error strings to Stem to be logged.
#[no_mangle]
pub extern "C" fn log_error(msg: *const c_char) {
    let msg = cstr_to_string(msg);
    tracing::error!("Swift error: {}", msg);
}

/// Handle shutdown of the app by saving certain persistent state elements to
/// the filesystem, which will be read-back at startup.
#[no_mangle]
pub extern "C" fn handle_shutdown() {
    tracing::info!("----- App Shutdown -----");
    app_state::AppState::save_to_file();
}

/// When the app comes back to the foreground we start the UI server, and pass
/// up the new configuration. This way our frontend key rotates every time the
/// app is brought to the foreground.
#[no_mangle]
pub extern "C" fn handle_enter_foreground() -> server::ServerConfig {
    tracing::info!("App Foregrounded");
    runtime::get_runtime().block_on(async {
        // We may have received new data while in the background
        tokio::spawn(geojson::update_geojson(None, true));
        tokio::spawn(map::automap::update_automap());
        tokio::spawn(async {
            if let Err(e) = logs::update_last_logged_error().await {
                tracing::error!(
                    "IO failure when updating last logged error: {e}"
                );
            };
        });
        server::run(0, true).await
    })
}

/// When the app goes the background we stop the server. This way we release
/// resources that were in use, can better handle times when the OS kills worker
/// threads, and reduce the opportunities for other programs to connect to the
/// server and read out private data (the secret key given to the frontend also
/// helps).
#[no_mangle]
pub extern "C" fn handle_enter_background() {
    tracing::info!("App Backgrounded");
    runtime::get_runtime().block_on(async {
        server::shutdown().await;
        database::reduce_db_cache_size().await;
    });
    // save the app state to file
    app_state::AppState::save_to_file();
}

/// Log a location in the app. This is a thin sync wrapper around the helper
/// function in `core`.
#[no_mangle]
pub extern "C" fn log_location(loc: database::OSLocationData) {
    runtime::get_runtime().block_on(async {
        core::log_location(loc).await;
    });
}

/// Return whether the standard location service should be enabled
///
/// Note: not safe to call within a tokio runtime!
#[no_mangle]
pub extern "C" fn get_location_enabled() -> bool {
    runtime::get_runtime().block_on(async {
        location_config::get_standard_location_enabled().await
    })
}

/// Return whether we should enabled the significant location changes service
#[no_mangle]
pub extern "C" fn get_significant_changes() -> bool {
    location_config::get_significant_changes_enabled()
}

/// Return the distance filter setting (used for standard location service)
#[no_mangle]
pub extern "C" fn get_distance_filter() -> f32 {
    location_config::get_distance_filter()
}

/// Return the location accuracy mode (used for standard location service)
#[no_mangle]
pub extern "C" fn get_location_accuracy_mode() -> common::LocationAccuracyMode {
    location_config::get_location_accuracy_mode()
}

/// Tell swift to export the SQLite log in a share sheet
#[no_mangle]
pub extern "C" fn should_export_sqlite_log() -> bool {
    let state = app_state::AppState::global();
    let mut guard = state.swift_messages.lock().unwrap();
    let should_export = guard.should_export_sqlite_log;
    // unset the setting if it was true
    guard.should_export_sqlite_log = false;
    if should_export {
        // Checkpoint the database so all outstanding transactions move from the
        // WAL file to the database
        runtime::get_runtime().block_on(async {
            database::checkpoint_db().await;
        });
    }
    should_export
    // drop the lock guard
}

/// Tell swift to import a SQLite log with a document picker
#[no_mangle]
pub extern "C" fn should_import_sqlite_log() -> bool {
    let state = app_state::AppState::global();
    let mut guard = state.swift_messages.lock().unwrap();
    let should_import = guard.should_import_sqlite_log;
    guard.should_import_sqlite_log = false;
    should_import
}

#[no_mangle]
pub extern "C" fn import_from_sqlite_log(import_path: *const c_char) {
    let import_path = PathBuf::from(cstr_to_string(import_path));
    runtime::get_runtime().block_on(async {
        database::import_database_records(import_path).await
    })
}

/// Tell swift to share the SQLite log in a share sheet
#[no_mangle]
pub extern "C" fn should_request_when_in_use_authorization() -> bool {
    let state = app_state::AppState::global();
    let mut guard = state.swift_messages.lock().unwrap();
    let should_request = guard.should_request_when_in_use_authorization;
    guard.should_request_when_in_use_authorization = false;
    should_request
}

/// Tell swift to share the generated track_export.{ext} track in a share sheet
#[no_mangle]
pub extern "C" fn should_export_track() -> bool {
    let state = app_state::AppState::global();
    let mut guard = state.swift_messages.lock().unwrap();
    let should_export = guard.should_export_track;
    guard.should_export_track = false;
    should_export
}

/// Unit tests for the top-level library interface.
#[cfg(test)]
pub mod tests {
    use std::ffi::CString;

    /// Test the top level C interface.
    #[test]
    fn test_app_dir_update() {
        // setup the file system
        let paths = super::local::local_fs_setup("test_app_dir_update/");
        let subdirs = vec![
            paths.documents_dir.clone(),
            paths.library_dir.clone(),
            paths.temp_dir.clone(),
            paths.bundle_dir.clone(),
        ];
        // construct our cstrings
        let cstrings: Vec<_> = subdirs
            .iter()
            .map(|s| CString::new(&*s.to_string_lossy()).unwrap())
            .collect();
        let version = CString::new("test").unwrap();
        // call the C-facing set_app_dirs function
        super::set_app_dirs(
            cstrings[0].as_ptr(),
            cstrings[1].as_ptr(),
            cstrings[2].as_ptr(),
            cstrings[3].as_ptr(),
            version.as_ptr(),
        );
        // test that we can now get the Documents directory as expected
        assert_eq!(super::paths::get_documents_dir(), paths.documents_dir);
    }
}

/// Local setup either for development or testing.
/// Not used in any production app code. TODO: gate with feature flag
pub mod local {
    use super::init;
    use super::paths::Paths;
    use super::server;

    use std::fs;
    use std::path::PathBuf;

    /// Set the directories to use for the test given a top level directory.
    /// We use an OS-assigned port so the tests can be run in parallel.
    pub async fn test_setup(dir: &str) -> u16 {
        local_setup(dir, 0).await
    }

    /// Sets up a local filesystem and initializes stem with the provided port.
    /// Returns the actual port used to the caller. Local development is
    /// insecure; the scope/secret key is always set to "123". Assume the
    /// caller knows this already so we just return the port.
    pub async fn local_setup(dir: &str, port: u16) -> u16 {
        let paths = local_fs_setup(dir);
        init(paths, "1.test.0".into()).await;
        server::run(port, false).await.port
    }

    /// Sets up a local filesystem and initializes stem with the provided port,
    /// but copies the development database over to the new filesystem. This is
    /// useful for debugging on collected data. Just Take the SQLite database
    /// from the device, and put it in place of the development database at
    /// stem/db/data.db.
    /// Returns the actual port used to the caller.
    pub async fn local_setup_with_dev_db(dir: &str, port: u16) -> u16 {
        // save the previous state to a temp file, then put it back, ignoring
        // any errors with `let _ =`
        let state_file =
            PathBuf::from(dir).join("Library/persistent_state.json");
        let tmp_file = "persistent_state.json";
        let _ = std::fs::copy(&state_file, tmp_file);
        let paths = local_fs_setup(dir); // this clears the previous contents
        let _ = std::fs::copy(tmp_file, &state_file);
        let _ = std::fs::remove_file(tmp_file);
        copy_dev_db(paths.documents_dir.clone());
        init(paths, "1.test.0".into()).await;
        server::run(port, false).await.port
    }

    /// Set up a directory for local testing. Provided argument is the name of
    /// the directory to have the filesystem under. This function clears it out
    /// if it already has contents, recreates it, and returns the paths.
    pub fn local_fs_setup(dir: &str) -> Paths {
        clear_directory(dir);
        create_subdirs(dir)
    }

    /// Clear the test directory from before if it remains. This gets us to a
    /// known state for each test
    fn clear_directory(dir: &str) {
        if std::fs::metadata(dir).is_ok() {
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    /// Create the subdirectories needed for testing the app and return a vector
    /// of the subdirectory paths
    fn create_subdirs(dir: &str) -> Paths {
        let dir = PathBuf::from(dir);
        let subdirs = vec!["Documents", "Library", "tmp", "Bundle"];
        let fullsubdirs: Vec<_> =
            subdirs.iter().map(|subdir| dir.join(subdir)).collect();
        for fullsubdir in &fullsubdirs {
            std::fs::create_dir_all(fullsubdir).unwrap();
        }
        Paths {
            documents_dir: fullsubdirs[0].clone(),
            library_dir: fullsubdirs[1].clone(),
            temp_dir: fullsubdirs[2].clone(),
            bundle_dir: fullsubdirs[3].clone(),
        }
    }

    /// Copy the development database to the local filesystem being set up. This
    /// keeps data that was in the database, as opposed to creating a completely
    /// new one.
    fn copy_dev_db(documents_dir: std::path::PathBuf) {
        let dev_db_path = "src/app/stem/db/data.db";
        let dest = documents_dir.join("data.db");
        fs::copy(dev_db_path, dest).unwrap();
    }
}
