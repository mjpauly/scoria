//! Shared Tokio async runtime.
//!
//! Lazily initialized and guarded with an Arc.

use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

static RT: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(Builder::new_multi_thread().enable_all().build().unwrap())
});

/// Get a reference to the runtime.
///
/// Usage:
///     get_runtime().block_on(async {
///         func().await;
///     });
pub fn get_runtime() -> Arc<Runtime> {
    RT.clone()
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
