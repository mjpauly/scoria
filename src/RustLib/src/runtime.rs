use std::cell::RefCell;

use tokio::runtime::Runtime;

// Shared runtime
thread_local!(static RT: RefCell<Runtime> = RefCell::new(Runtime::new().unwrap()));

#[cfg(test)]
mod tests {

    #[test]
    fn test_get_rt() {
        assert_eq!(1, 1);
    }
}
