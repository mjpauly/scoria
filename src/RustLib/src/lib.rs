//! Core Epsilon library interface.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.

use std::ffi::CStr;
use std::fs::File;
use std::io::prelude::*;
use std::os::raw::c_char;

use rand::prelude::*;

mod db;
mod paths;
mod rt;

// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String {
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Set the documents directory known to the Rust library.
///
/// This should be called first thing when the app is launched.
#[no_mangle]
pub extern "C" fn set_documents_dir(dir: *const c_char) {
    paths::set_storage_dir(cstr_to_string(dir));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_storage_dir_update() {
        assert!(paths::get_storage_dir().is_err());
        let dir = CString::new("./").unwrap();
        set_documents_dir(dir.as_ptr());
        let expected = dir.into_string().unwrap();
        assert_eq!(paths::get_storage_dir().unwrap(), expected);
    }
}

#[no_mangle]
pub extern "C" fn log_location(lat: f64, lon: f64, accuracy: f64, speed: f64, course: f64) -> i32 {
    db::log_location(lat, lon, accuracy, speed, course);
    0
}

// TODO: functions to implement
// log location
// make heatmap viz
// create marker (space / time)
//      lookup marker from public DB (apple maps?, openstreetmap?)
// perform queries
//      visits (last time, first time, total, time spent, when visits happen)
//      traveling (different modes, time spent, num trips, when it happens)
//      trends

#[no_mangle]
pub extern "C" fn vec_print() -> i32 {
    let v = vec![1, 2, 3];
    println!("{:?}", v);
    0
}

#[no_mangle]
pub extern "C" fn rand_check() -> i32 {
    let x: u8 = random();
    println!("{}", x);
    assert!(x > 0);
    0
}

#[no_mangle]
pub extern "C" fn write_file(dir: *const c_char) {
    let mut filepath = cstr_to_string(dir);
    filepath.push_str("/foo.txt");

    println!("Writing file to: {:?}", filepath);
    let mut file = File::create(filepath).unwrap();
    file.write_all(b"Hello, world!").unwrap();
}

#[no_mangle]
pub extern "C" fn get_a_value_from_rust() -> i32 {
    println!("printing from rust!");
    42
}
