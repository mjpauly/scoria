//! Haptic feedback, fired straight from the webview into the native layer
//! through the native_haptic javascript hook. Unlike app state, which flows
//! through the backend, haptics are fire-and-forget UI events, so they skip
//! the backend entirely. In a plain browser the hook is a no-op.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn native_haptic(kind: &str);
}

/// Warm up the haptic engine at the start of a gesture so the first tick
/// isn't late or dropped. No-op on Android.
pub fn prepare() {
    native_haptic("prepare");
}

/// A subtle detent tick, for scrubbing across a step boundary.
pub fn tick() {
    native_haptic("tick");
}

/// A firmer impact for a press-and-hold registering, matching the system
/// long-press feel.
pub fn hold() {
    native_haptic("hold");
}
