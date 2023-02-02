//! Shared Tokio async runtime.
//!
//! For now a new runtime is instantiated for each call to lib.rs

use std::rc::Rc;

use tokio::runtime::Runtime;

// Shared runtime
// Rc provides a reference counted pointer without mutability.
thread_local!(static RT: Rc<Runtime> = Rc::new(Runtime::new().unwrap()));

/// Get a reference to the runtime.
///
/// Usage:
///     get_runtime().block_on(async {
///         func().await;
///     });
pub fn get_runtime() -> Rc<Runtime> {
    RT.with(|rt| Rc::clone(rt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rt() {
        assert_eq!(1, 1);
        get_runtime().block_on(async {
            assert_eq!(1, 1);
        });
    }
}
