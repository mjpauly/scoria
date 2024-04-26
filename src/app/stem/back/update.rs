//! Android-only module for checking for app updates.

use tracing::error;

use crate::app_state::{get_back_state, set_back_state};

/// If we've just updated, we'll set the available app version back to None.
fn clear_available_if_updated(current_version_code: i64) {
    let saved_available_version =
        get_back_state(|back| back.available_app_version.clone());
    if let Some(saved) = saved_available_version {
        if saved.0 == current_version_code {
            set_back_state(|back| back.available_app_version = None);
        }
    }
}

pub async fn check_for_update(current_version_code: i64) {
    clear_available_if_updated(current_version_code);
    let client = reqwest::ClientBuilder::new().build().unwrap();
    let result = client
        .get("https://scoria.info/download/apk/latest_version")
        .send()
        .await;
    let response = match result {
        Ok(resp) => resp,
        // could be unreachable due to being offline, not worth putting an error
        // in the log
        Err(_) => return,
    };
    if !response.status().is_success() {
        error!(
            "Received non-sucess status code {} when checking the latest \
            available app version. Check https://scoria.info for more info.",
            response.status(),
        );
        return;
    }
    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to retrieve response body. Error: {e}");
            return;
        }
    };
    let mut split = text.split(',');
    let (ver_code, ver_name) = match (split.next(), split.next()) {
        (Some(c), Some(n)) => (c, n),
        _ => {
            error!(
                "Failed to decode version code and version name from {text}."
            );
            return;
        }
    };
    let available_version_code = match ver_code.parse::<i64>() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse version code {ver_code} with error: {e}");
            return;
        }
    };
    if current_version_code < available_version_code {
        set_back_state(|back| {
            back.available_app_version =
                Some((available_version_code, ver_name.to_string()))
        });
    } else {
        set_back_state(|back| back.available_app_version = None);
    }
}
