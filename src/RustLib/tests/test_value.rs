extern crate lib;

use std::ffi::CString;

use lib::get_a_value_from_rust;
use lib::vec_print;
use lib::rand_check;
use lib::write_file;

#[test]
fn test_value() {
    let value = get_a_value_from_rust();
    // assert_eq!(value, 42);
    assert_eq!(value, 42);
}

#[test]
fn test_std() {
    let out = vec_print();
    let v = rand_check();
    println!("{:?}", v);
    assert_eq!(0, out);
}

#[test]
fn test_write_file() {
    let s = CString::new("./").unwrap();
    write_file(s.as_ptr());
}
