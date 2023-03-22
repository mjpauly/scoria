//! Core Epsilon library interface.
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

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

pub mod app_state; // backend state storage
#[path = "../front/rs/common.rs"]
pub mod common; // defs in common with frontend (e.g. messages)
pub mod core; // high-level app logic that spans multiple modules
pub mod database; // manages the SQLite database
pub mod paths; // stores and retrieve file system paths
pub mod runtime; // retrieves async runtime for use in the sync C interface
pub mod server; // server for the UI
pub mod ws_session; // websocket actor for the UI

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
) -> u16 {
    let paths_to_set = paths::Paths {
        documents_dir: PathBuf::from(cstr_to_string(documents_dir)),
        library_dir: PathBuf::from(cstr_to_string(library_dir)),
        temp_dir: PathBuf::from(cstr_to_string(temp_dir)),
        bundle_dir: PathBuf::from(cstr_to_string(bundle_dir)),
    };
    runtime::get_runtime().block_on(async { init(paths_to_set).await })
}

/// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Initialize with port 0, which means the OS will assign us a free port.
pub async fn init(init_paths: paths::Paths) -> u16 {
    init_with_port(init_paths, 0).await
}

/// Initializes the rust library with the given app directories.
pub async fn init_with_port(init_paths: paths::Paths, port: u16) -> u16 {
    let db = database::init_db(paths::get_db_path_helper(
        init_paths.documents_dir.clone(),
    ))
    .await
    .unwrap();
    let port = server::run(init_paths.clone(), "127.0.0.1", port);
    app_state::AppState::init(init_paths, db);
    port
}

/// Log a location in the app. This is a thin sync wrapper around the helper
/// function in `core`.
#[no_mangle]
pub extern "C" fn log_location(
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    datetime_epoch: i64,
) {
    runtime::get_runtime().block_on(async {
        core::log_location(lat, lon, accuracy, speed, course, datetime_epoch)
            .await;
    });
}

/// Return the distance filter setting
#[no_mangle]
pub extern "C" fn get_distance_filter() -> f32 {
    return *app_state::AppState::global()
        .distance_filter
        .lock()
        .unwrap();
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
        // call the C-facing set_app_dirs function
        super::set_app_dirs(
            cstrings[0].as_ptr(),
            cstrings[1].as_ptr(),
            cstrings[2].as_ptr(),
            cstrings[3].as_ptr(),
        );
        // test that we can now get the Documents directory as expected
        assert_eq!(super::paths::get_documents_dir(), paths.documents_dir);
    }
}

/// Local setup either for development or testing.
/// Not used in any production app code. TODO: gate with feature flag
pub mod local {
    use super::init_with_port;
    use super::paths::Paths;

    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// Set the directories to use for the test given a top level directory.
    /// We use an OS-assigned port so the tests can be run in parallel.
    pub async fn test_setup(dir: &str) -> u16 {
        local_setup(dir, 0).await
    }

    /// Sets up a local filesystem and initializes stem with the provided port.
    /// Returns the actual port used to the caller.
    pub async fn local_setup(dir: &str, port: u16) -> u16 {
        let paths = local_fs_setup(dir);
        init_with_port(paths, port).await
    }

    /// Set up a directory for local testing. Provided argument is the name of
    /// the directory to have the filesystem under. This function clears it out
    /// if it already has contents, recreates it, copies the frontend zip bundle
    /// to the right spot, and returns the paths.
    pub fn local_fs_setup(dir: &str) -> Paths {
        clear_directory(dir);
        let paths = create_subdirs(dir);
        copy_bundle(paths.bundle_dir.clone()).unwrap();
        paths
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

    /// Copy the bundle into the right directory so the server can start up.
    fn copy_bundle(
        bundle_dir: std::path::PathBuf,
    ) -> Result<(), std::io::Error> {
        // Copy our bundle into the expected location
        let bundle_path = "src/app/stem/front/dist.zip";
        let dest = bundle_dir.join("dist.zip");
        fs::copy(bundle_path, dest.clone())?;
        // dist.zip is a bazel output, so it is read-only by default. We change
        // it to writable so that future invocations of fs::copy will work even
        // if the sandbox is not cleared, and also so that the read-only
        // permissions don't propagate further
        fs::set_permissions(dest.clone(), fs::Permissions::from_mode(0o666))?;
        println!(
            "Copied dist.zip & set permissions to: {:#o} (hopefully 0o100666)",
            fs::metadata(dest.clone())?.permissions().mode()
        );
        Ok(())
    }
}
