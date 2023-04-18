//! Shared Tokio async runtime.
//!
//! This can be a thread_local since calls to the Swift-facing C API always
//! occur on the same thread.

use std::rc::Rc;

use tokio::runtime::{Builder, Runtime};

// Shared runtime
// Rc provides a reference counted pointer without mutability.
thread_local!(static RT: Rc<Runtime> = Rc::new(
    Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
));

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
