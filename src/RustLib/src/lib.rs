use rand::prelude::*;

#[no_mangle]
pub extern fn get_a_value_from_rust() -> i32 {
    42
}

// #[no_mangle]
// pub extern fn log_location(lat: f32, lon: f32, accuracy: f32,
                           // speed: f32, course: f32, time: i64) -> i32 {
    // 0
// }

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

