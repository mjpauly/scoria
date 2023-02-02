extern crate lib;

use lib::get_a_value_from_rust;

#[test]
fn test_value() {
    let value = get_a_value_from_rust();
    assert_eq!(value, 42);
}
