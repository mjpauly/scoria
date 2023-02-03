//! Core Epsilon library interface.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.

use std::ffi::CStr;
use std::os::raw::c_char;

mod database;
mod paths;
mod runtime;
mod viz;

// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Initializes the rust library with the given storage directory.
async fn init(storage_dir: String) {
    paths::set_storage_dir(storage_dir);
    database::init_db().await.unwrap();
}

/// Set the documents directory known to the Rust library.
///
/// This should be called first thing when the app is launched.
#[no_mangle]
pub extern "C" fn set_documents_dir(dir: *const c_char) {
    runtime::get_runtime().block_on(async {
        init(cstr_to_string(dir)).await;
    });
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::ffi::CString;

    pub async fn test_setup(dir: &str) {
        std::fs::create_dir_all(dir).unwrap();
        init(String::from(dir)).await;
    }

    #[test]
    fn test_storage_dir_update() {
        assert!(paths::get_storage_dir().is_err());
        let dir = CString::new("./").unwrap();
        set_documents_dir(dir.as_ptr());
        let expected = dir.into_string().unwrap();
        assert_eq!(paths::get_storage_dir().unwrap(), expected);
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

/// Generate a visualization of the past week
#[no_mangle]
pub extern "C" fn gen_past_week_viz() {
    runtime::get_runtime().block_on(async {
        viz::gen_past_week_viz().await;
    });
}

/// Generate a visualization with the given options
#[no_mangle]
pub extern "C" fn gen_viz(
    datetime_epoch_start: i64,
    datetime_epoch_end: i64,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) {
    runtime::get_runtime().block_on(async {
        viz::gen_viz(datetime_epoch_start, datetime_epoch_end, r, g, b, a)
            .await;
    });
}
