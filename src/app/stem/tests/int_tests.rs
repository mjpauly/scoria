//! We can't run these as normal, parallelized tests since the backend is
//! multithreaded with a global state. So we run each tests sequentially under
//! a single integration_tests() function.
//!
//! Remember: This is where we draw the line between a unit test and integration
//! test. If it touches the global state from a different thread than the main
//! thread (e.g. when handling websocket requests in ws_session.rs), we can't
//! do it as a unit test.

extern crate stem;

mod back_interface;
mod front_interface;

use stem::local::test_setup;

/// Helper to do dut setup and return the websocket url
async fn setup(dir: &str) -> String {
    let port = test_setup(dir).await;
    format!("ws://127.0.0.1:{}/ws", port)
}
