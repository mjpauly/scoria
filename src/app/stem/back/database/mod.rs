pub mod location;
pub mod mounted;
pub mod pins;

use std::str::FromStr;

use anyhow::{Context, Result};
use common::mounted::MAIN_DB_MOUNT_ID;
pub use location::*;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

use crate::{
    app_state::{set_derived_state, AppState},
    logs::LogErrorAndContinue,
    paths::get_db_path,
};

pub async fn init_main_db() {
    let db_path = get_db_path();
    match open_db(db_path).await.context("opening main database") {
        Ok(conn) => {
            AppState::global()
                .dbs
                .lock()
                .unwrap()
                .insert(MAIN_DB_MOUNT_ID, conn);
            set_derived_state(|s| {
                s.mounted_dbs_on_disk.insert(MAIN_DB_MOUNT_ID, None)
            });
        }
        Err(e) => {
            tracing::error!("{e}");
            // let frontend know about the error tied to this db
            set_derived_state(|s| {
                s.mounted_dbs_on_disk
                    .insert(MAIN_DB_MOUNT_ID, Some(format!("{e:?}")));
            });
            return;
        }
    };
    if let Ok(conn) = get_main_db_pool() {
        // vacuum and checkpoint the database at startup, so it shrinks to size
        checkpoint_db(&conn).await;
    }
}

/// Open a database given its path. Call this at startup for the main database,
/// and at foregrounding for mounted databases.
pub async fn open_db(db_path: String) -> Result<SqlitePool> {
    let opt = SqliteConnectOptions::from_str(&db_path)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let conn = SqlitePool::connect_with(opt)
        .await
        .context("connecting to database")?;
    MIGRATOR.run(&conn).await.context("running migrations")?;
    sqlx::query("PRAGMA secure_delete = on;")
        .execute(&conn)
        .await
        .context("setting secure delete on")
        .log_error_and_continue();
    Ok(conn)
}

/// Increase the cache sizes for all databases on foregrounding.
pub async fn increase_db_cache_sizes() {
    let dbs = AppState::global().dbs.lock().unwrap().clone();
    for (id, db) in dbs.iter() {
        increase_db_cache_size(db)
            .await
            .with_context(|| format!("increasing cache size of database {id}"))
            .log_error_and_continue();
    }
}

/// Decrease the cache sizes for all databases on backgrounding.
pub async fn reduce_db_cache_sizes() {
    let dbs = AppState::global().dbs.lock().unwrap().clone();
    for (id, db) in dbs.iter() {
        reduce_db_cache_size(db)
            .await
            .with_context(|| format!("reducing cache size of database {id}"))
            .log_error_and_continue();
    }
}

/// Increase the database cache size when in the foreground to 2Gb (2M
/// kibibytes)
async fn increase_db_cache_size(conn: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA cache_size = -2000000;")
        .execute(conn)
        .await?;
    Ok(())
}

/// Lower the database cache size when in the background to 2Mb (2k kibibytes)
async fn reduce_db_cache_size(conn: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA cache_size = -2000;")
        .execute(conn)
        .await?;
    Ok(())
}
