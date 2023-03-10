//! Core Epsilon library interface.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

use actix_web::dev::ServerHandle;

pub mod app_state;
pub mod database;
pub mod paths;
pub mod runtime;
pub mod startup;
pub mod viz;
pub mod ws_session;

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
    let paths_to_set = paths::Paths {
        documents_dir: Some(PathBuf::from(cstr_to_string(documents_dir))),
        library_dir: Some(PathBuf::from(cstr_to_string(library_dir))),
        temp_dir: Some(PathBuf::from(cstr_to_string(temp_dir))),
        bundle_dir: Some(PathBuf::from(cstr_to_string(bundle_dir))),
    };
    runtime::get_runtime().block_on(async {
        init(paths_to_set).await.unwrap();
    });
}

// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Initializes the rust library with the given app directories.
pub async fn init(paths_to_set: paths::Paths) -> Result<ServerHandle, String> {
    paths::set_app_dirs(paths_to_set);
    database::init_db().await.unwrap();
    startup::run("127.0.0.1", 8081)
}

/// Local setup either for development or testing.
pub mod local {
    use super::*;

    /// Create the subdirectories needed for testing the app and return a vector
    /// of the subdirectory paths
    pub fn create_subdirs(dir: &str) -> paths::Paths {
        let dir = PathBuf::from(dir);
        let subdirs = vec!["Documents", "Library", "tmp", "Bundle"];
        let fullsubdirs: Vec<_> =
            subdirs.iter().map(|subdir| dir.join(subdir)).collect();
        for fullsubdir in &fullsubdirs {
            std::fs::create_dir_all(fullsubdir).unwrap();
        }
        paths::Paths {
            documents_dir: Some(fullsubdirs[0].clone()),
            library_dir: Some(fullsubdirs[1].clone()),
            temp_dir: Some(fullsubdirs[2].clone()),
            bundle_dir: Some(fullsubdirs[3].clone()),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::ffi::CString;

    use local::create_subdirs;

    /// Set the directories to use for the test given a top level directory.
    pub async fn test_setup(dir: &str) {
        let paths_to_set = create_subdirs(dir);
        // init(paths_to_set).await;
        paths::set_app_dirs(paths_to_set);
        database::init_db().await.unwrap();
    }

    /// Run test_setup but clear the contents of the test directory first
    pub async fn test_setup_clean(test_dir: &str) {
        if std::fs::metadata(test_dir).is_ok() {
            // need to clear it manually if left over from previous test
            std::fs::remove_dir_all(test_dir).unwrap();
        }
        test_setup(test_dir).await;
    }

    /// Test the top level C interface.
    ///
    /// This test is ignored by default, since it does not set up the app
    /// directories and populate them with the necessary contents for a call to
    /// `init(paths_to_set).await.unwrap()` to succeed. Remove the `unwrap()` to
    /// see the test pass.
    #[ignore]
    #[test]
    fn test_app_dir_update() {
        assert!(paths::get_documents_dir().is_err());
        create_subdirs("");
        let subdirs = vec!["Documents", "Library", "tmp", "Bundle"];
        let cstrings: Vec<_> =
            subdirs.iter().map(|s| CString::new(*s).unwrap()).collect();
        set_app_dirs(
            cstrings[0].as_ptr(),
            cstrings[1].as_ptr(),
            cstrings[2].as_ptr(),
            cstrings[3].as_ptr(),
        );
        assert_eq!(
            paths::get_documents_dir().unwrap(),
            PathBuf::from("Documents")
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
