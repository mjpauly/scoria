//! Simulated safe-area insets for debug builds, so screenshots taken in
//! Firefox's device simulation (where env(safe-area-inset-*) is 0 and nothing
//! injects --safe-*) keep the pre-env() hardcoded spacing. Only a fallback:
//! environments that already resolve real insets are left untouched.

use web_sys::window;

/// If the safe-area insets all resolve to zero, enable the .sim-insets
/// values defined in styles.css.
pub fn apply_fallback() {
    let Some(win) = window() else { return };
    let Some(doc) = win.document() else { return };
    let (Some(root), Some(body)) = (doc.document_element(), doc.body()) else {
        return;
    };

    // Probe the resolved insets by measuring an element sized by their sum
    let Ok(probe) = doc.create_element("div") else {
        return;
    };
    if probe
        .set_attribute(
            "style",
            "position:fixed;\
             height:calc(var(--safe-area-top) + var(--safe-area-bottom));",
        )
        .is_err()
        || body.append_child(&probe).is_err()
    {
        return;
    }
    let height = win
        .get_computed_style(&probe)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("height").ok())
        .and_then(|h| h.trim_end_matches("px").parse::<f64>().ok());
    probe.remove();

    if height == Some(0.0) {
        // set_class_name instead of class_list: the DomTokenList web-sys
        // feature isn't enabled, and nothing else puts classes on <html>
        root.set_class_name("sim-insets");
    }
}
