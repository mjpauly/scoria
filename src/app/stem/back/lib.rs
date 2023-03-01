//! Core Epsilon library interface.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

mod app_state;
mod database;
mod paths;
mod runtime;
mod startup;
mod viz;

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
) {
    let paths_to_set = paths::Paths::new(
        Some(PathBuf::from(cstr_to_string(documents_dir))),
        Some(PathBuf::from(cstr_to_string(library_dir))),
        Some(PathBuf::from(cstr_to_string(temp_dir))),
        Some(PathBuf::from(cstr_to_string(bundle_dir))),
    );
    runtime::get_runtime().block_on(async {
        init(paths_to_set).await;
    });
}

// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Initializes the rust library with the given app directories.
async fn init(paths_to_set: paths::Paths) {
    paths::set_app_dirs(paths_to_set);
    database::init_db().await.unwrap();
    startup::run("127.0.0.1", 8081);
}

/// Local setup either for development or testing.
pub mod local {
    use super::*;

    /// Set the directories to use for the test given a top level directory.
    pub async fn local_setup(dir: &str) {
        let fullsubdirs = create_subdirs(dir);
        let paths_to_set = paths::Paths::new(
            Some(fullsubdirs[0].clone()),
            Some(fullsubdirs[1].clone()),
            Some(fullsubdirs[2].clone()),
            Some(fullsubdirs[3].clone()),
        );
        init(paths_to_set).await;
    }

    /// Create the subdirectories needed for testing the app and return a vector
    /// of the subdirectory paths
    pub fn create_subdirs(dir: &str) -> Vec<PathBuf> {
        let dir = PathBuf::from(dir);
        let subdirs = vec!["Documents", "Library", "tmp", "Bundle"];
        let fullsubdirs: Vec<_> =
            subdirs.iter().map(|subdir| dir.join(subdir)).collect();
        for fullsubdir in &fullsubdirs {
            std::fs::create_dir_all(fullsubdir).unwrap();
        }
        fullsubdirs
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::ffi::CString;

    use local::create_subdirs;
    pub use local::local_setup as test_setup;

    /// Run test_setup but clear the contents of the test directory first
    pub async fn test_setup_clean(test_dir: &str) {
        if std::fs::metadata(test_dir).is_ok() {
            // need to clear it manually if left over from previous test
            std::fs::remove_dir_all(test_dir).unwrap();
        }
        test_setup(test_dir).await;
    }

    /// Test the top level C interface
    #[test]
    fn test_app_dir_update() {
        assert!(paths::get_documents_dir().is_err());
        let fullsubdirs = create_subdirs("./");
        let cstrings: Vec<_> = fullsubdirs
            .iter()
            .map(|s| {
                CString::new(
                    (s.clone()).into_os_string().into_string().unwrap(),
                )
                .unwrap()
            })
            .collect();
        set_app_dirs(
            cstrings[0].as_ptr(),
            cstrings[1].as_ptr(),
            cstrings[2].as_ptr(),
            cstrings[3].as_ptr(),
        );
        assert_eq!(
            paths::get_documents_dir().unwrap(),
            PathBuf::from("./Documents")
        );
    }
}

/// Log a location into the SQLite database
#[no_mangle]
pub extern "C" fn log_location(
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    datetime_epoch: i64,
) -> i32 {
    runtime::get_runtime().block_on(async {
        let result = database::log_location(
            lat,
            lon,
            accuracy,
            speed,
            course,
            datetime_epoch,
        )
        .await;
        match result {
            Err(e) => println!("Failed to log location due to error: {}", e),
            _ => (),
        };
    });
    0
}
