//! Shared Tokio async runtime.
//!
//! For now a new runtime is instantiated for each call to lib.rs

use std::cell::RefCell;
use std::rc::Rc;

use tokio::runtime::Runtime;

// Shared runtime
thread_local!(static RT: Rc<RefCell<Runtime>> =
              Rc::new(RefCell::new(Runtime::new().unwrap())));

/// Get a reference to the runtime.
///
/// Usage:
///     let binding = get_runtime_binding();
///     let rt = binding.borrow();
pub fn get_runtime_binding() -> Rc<RefCell<Runtime>> {
    RT.with(|rt| Rc::clone(rt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rt() {
        assert_eq!(1, 1);
        let binding = get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            println!("inside runtime");
            assert_eq!(1, 1);
        });
    }
}
