
// use std::cell::RefCell;

// use tokio::runtime::Runtime;
//
// // Our shared runtime.
// thread_local!(static RT: RefCell<Ctx> = RefCell::new(Runtime::new().unwrap()));
//
// #[cfg(test)]
// mod tests {
// use super::get_db_pool;
//
// #[test]
// fn test_get_rt() {
// assert_eq!(1, 1);
// }
// }
