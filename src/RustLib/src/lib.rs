//! Core Epsilon functionality.
//!
//! Responsible for database interactions, visualizations, and server
//! communication.

use rand::prelude::*;
use std::fs::File;
use std::io::prelude::*;

use std::ffi::CStr;  // accepting string pointers
use std::os::raw::c_char;

use std::cell::RefCell;

static DB_FNAME: &str = "mydata.sqlite";

// Our state data is stored in CTX
// We don't know the contents during runtime initialization so it starts empty
thread_local!(static CTX: RefCell<Ctx> = RefCell::new(Ctx::empty()));

/// Context struct that we store in the library containing the database path.
///
/// Lets us keep the value without having to pass it through every function
/// call.
struct Ctx {
    db_path: Option<String>,
}

impl Ctx {
    /// Create an empty context with no db_path since it's unknown at startup.
    fn empty() -> Ctx {
        Ctx {
            db_path: None,
        }
    }

    /// Update the db_path in the context with a new documents directory.
    fn update_db_path(&mut self, documents_dir: String) {
        let mut new_path = documents_dir;
        new_path.push_str(DB_FNAME);  // TODO: safe path appending
        self.db_path = Some(new_path);
    }
}

/// Convert a const char* reference from C into an owned Rust String.
fn cstr_to_string(cstr: *const c_char) -> String{
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    String::from_utf8_lossy(cstr.to_bytes()).to_string()
}

/// Update the documents directory known to the library.
#[no_mangle]
pub fn update_documents_dir(dir: *const c_char) {
    CTX.with(|ctx| {
        (*ctx.borrow_mut())
            .update_db_path(cstr_to_string(dir));
    });
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;

    #[test]
    fn test_ctx_update() {
        let s = CString::new("./").unwrap();
        CTX.with(|ctx| {
            assert!((*ctx.borrow())
                    .db_path.is_none());
            update_documents_dir(s.as_ptr());
            let mut expected = s.into_string().unwrap();
            expected.push_str(DB_FNAME);
            assert_eq!(
                (*ctx.borrow()).db_path,
                Some(String::from("./mydata.sqlite")));
        });
    }
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
pub extern fn log_location(lat: f64, lon: f64, accuracy: f64,
                           speed: f64, course: f64) -> i32 { // TODO: time
    // arguments are received as 64-bit floaties, only lat/lon can't be
    // truncated to 32-bit without (potentially) losing some accuracy,
    // truncation should happen before storing in DB
    println!("{:?}, {:?}, {:?}, {:?}, {:?}",
             lat, lon, accuracy, speed, course);
    0
}

#[no_mangle]
pub extern fn vec_print() -> i32 {
    let v = vec![1, 2, 3];
    println!("{:?}", v);
    0
}

#[no_mangle]
pub extern fn rand_check() -> i32 {
    let x: u8 = random();
    println!("{}", x);
    assert!(x > 0);
    0
}

#[no_mangle]
pub extern fn write_file(dir: *const c_char) {
    let mut filepath = cstr_to_string(dir);
    filepath.push_str("/foo.txt");

    println!("Writing file to: {:?}", filepath);
    let mut file = File::create(filepath).unwrap();
    file.write_all(b"Hello, world!").unwrap();
}

#[no_mangle]
pub extern fn get_a_value_from_rust() -> i32 {
    println!("printing from rust!");
    42
}

