extern crate lib;

use lib::get_a_value_from_rust;
use lib::vec_print;
use lib::rand_check;

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

