//! Export the current map view as an image by taking the bytes and saving them
//! to file.

use std::fs::OpenOptions;
use std::io::Write;

use anyhow::Context;
use common::ToFront;
use tracing::error;

use crate::app_state::AppState;
use crate::{paths, ws_session};

pub fn export_image(data: &[u8]) {
    let fname = paths::get_image_export_fname();

    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true) // delete previous contents that are longer
        .open(fname)
        .context("opening image file for writing")
    {
        Ok(file) => file,
        Err(e) => {
            error!("{e:?}");
            return;
        }
    };
    if let Err(e) = file.write_all(data).context("writing image data to file") {
        error!("{e:?}");
        return;
    };
    AppState::global()
        .wrapper_messages
        .lock()
        .unwrap()
        .should_export_image = true;
    ws_session::send_message_to_front(ToFront::SwiftPoke);
}
