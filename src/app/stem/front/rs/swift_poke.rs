//! Poke the native Swift code that is listening to messages coming through the
//! WKWebView. With the poke it can then load new config settings from the
//! backend.

use std::time::Duration;
use wasm_bindgen::prelude::*;
use yew::platform::{spawn_local, time::sleep};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn webkit_poke();
}

pub fn poke() {
    spawn_local(async {
        // In testing the swift code beats the websocket update code by 1-8 ms.
        // We set this delay to 50 ms to have plenty of margin while maintaining
        // responsiveness.
        sleep(Duration::from_millis(50)).await;
        webkit_poke();
    });
}
