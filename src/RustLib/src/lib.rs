use rand::prelude::*;
use std::fs::File;
use std::io::prelude::*;

use std::ffi::CStr;  // accepting string pointers
use std::os::raw::c_char;

use std::thread;  // thread testing
use std::time::Duration;

#[no_mangle]
pub extern fn get_a_value_from_rust() -> i32 {
    println!("printing from rust!");
    42
}

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
    // let mut full = String::from(dir);
    let dir: &CStr = unsafe { CStr::from_ptr(dir) };
    let dir: &str = dir.to_str().unwrap();
    let mut dir: String = dir.to_owned();  // if necessary
    dir.push_str("/foo.txt");
    println!("Writing file to: {:?}", dir);
    let mut file = File::create(dir).unwrap();
    file.write_all(b"Hello, world!").unwrap();
}


#[no_mangle]
pub extern fn thread_test() {
    thread::spawn(|| {
        for i in 1..10 {
            println!("hi number {} from the spawned thread!", i);
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("hi number {} from the main thread!", i);
        thread::sleep(Duration::from_millis(1));
    }
}
