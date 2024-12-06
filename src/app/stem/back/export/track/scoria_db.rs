//! Export selected data as Scoria database.

use std::path::PathBuf;

use crate::{database, ws_session::send_error_popup};

use super::ExportData;

pub(super) async fn export(mut fname: PathBuf, data: ExportData) {
    fname.set_extension("scoria");
    let conn = match database::open_db(fname.display().to_string()).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to create export database: {e}");
            send_error_popup("Failed to create export database.");
            return;
        }
    };

    let mut first_err = None;
    for rec in data.records {
        if let Err(e) = database::log_location_with_db(rec, &conn).await {
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
    }
    if let Some(e) = first_err {
        tracing::error!("Failed to insert record(s) into export database: {e}");
        send_error_popup("Failed to insert record(s) into export database.");
    }
    // flush changes from WAL file to main database file
    database::checkpoint_db(&conn).await;
    conn.close().await;
}
