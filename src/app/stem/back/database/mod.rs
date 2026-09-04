pub mod grid;
pub mod location;
pub mod mounted;
pub mod pins;

use std::str::FromStr;

use anyhow::{Context, Result};
use common::mounted::{MountID, MAIN_DB_MOUNT_ID};
use common::state::DbStatus;
use futures_core::future::BoxFuture;
pub use location::*;
use sqlx::{
    error::BoxDynError,
    migrate::{Migration, MigrationSource, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

use crate::{
    app_state::{set_derived_state, AppState},
    logs::LogErrorAndContinue,
    paths::get_db_path,
};

/// Open the main database in the background. The pool lands in
/// AppState.dbs once migrations and the grid backfill are done; until then
/// get_main_db_pool fails, locations are queued (log_location) and the
/// frontend shows the status from mounted_dbs_on_disk.
pub async fn init_main_db() {
    set_derived_state(|s| {
        s.mounted_dbs_on_disk.insert(
            MAIN_DB_MOUNT_ID,
            DbStatus::Opening {
                stage: "starting".into(),
                progress: None,
            },
        )
    });
    let open = async {
        open_main_db().await;
        // the UI may be up already; refresh everything that read an
        // unopened database
        if AppState::global().ws_addr.lock().unwrap().is_some() {
            crate::core::update_on_foregrounding();
        }
    };
    // the test AppState is thread-local, so a spawned open can't see it
    #[cfg(test)]
    open.await;
    #[cfg(not(test))]
    tokio::spawn(open);
}

/// Open the main database, reporting status to the derived state.
pub async fn open_main_db() {
    let db_path = get_db_path();
    let result = open_db_with_status(db_path, MAIN_DB_MOUNT_ID)
        .await
        .context("opening main database");
    let status = match result {
        Ok(conn) => {
            // vacuum and checkpoint before publishing the pool: nothing
            // else can hold a read snapshot yet, so the checkpoint can't
            // be blocked, and the file shrinks before the UI queries it
            set_derived_state(|s| {
                s.mounted_dbs_on_disk.insert(
                    MAIN_DB_MOUNT_ID,
                    DbStatus::Opening {
                        stage: "compacting".into(),
                        progress: None,
                    },
                )
            });
            checkpoint_db(&conn).await;
            publish_main_db(conn).await;
            DbStatus::Ready
        }
        Err(e) => {
            tracing::error!("{e:?}");
            DbStatus::Error(format!("{e:?}"))
        }
    };
    set_derived_state(|s| {
        s.mounted_dbs_on_disk.insert(MAIN_DB_MOUNT_ID, status)
    });
}

/// Insert the locations that arrived while the main database was opening,
/// then make the pool available. The pool is published under the pending
/// lock once the queue is empty, and log_location queues under the same
/// lock, so no location is left in the queue afterwards and rowid order
/// stays time order.
async fn publish_main_db(conn: SqlitePool) {
    loop {
        let state = AppState::global();
        let pending = {
            let mut pending = state.pending_locations.lock().unwrap();
            if pending.is_empty() {
                state.dbs.lock().unwrap().insert(MAIN_DB_MOUNT_ID, conn);
                return;
            }
            std::mem::take(&mut *pending)
        };
        tracing::info!(
            "logging {} locations queued during open",
            pending.len()
        );
        let mut tx = match conn.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("flushing queued locations: {e}");
                continue;
            }
        };
        for loc in pending {
            if let Err(e) = log_location_with_db(loc, &mut *tx).await {
                tracing::error!("logging queued location: {e}");
            }
        }
        tx.commit()
            .await
            .context("committing queued locations")
            .log_error_and_continue();
    }
}

/// Open a database given its path, reporting the open's progress to the
/// derived-state status of mount `id`.
pub async fn open_db_with_status(
    db_path: String,
    id: MountID,
) -> Result<SqlitePool> {
    open_db_inner(db_path, |stage, progress| {
        if progress.is_none() {
            tracing::info!("opening db {id}: {stage}");
        }
        set_derived_state(|s| {
            s.mounted_dbs_on_disk.insert(
                id,
                DbStatus::Opening {
                    stage: stage.to_string(),
                    progress,
                },
            )
        });
    })
    .await
}

/// Open a database given its path, migrating it to the current schema.
/// Call this for databases nobody is waiting on in the UI (exports, tests);
/// the app's databases go through open_db_with_status.
pub async fn open_db(db_path: String) -> Result<SqlitePool> {
    open_db_inner(db_path, |_, _| {}).await
}

/// Version of the migration that builds the grid indexes. Migrations
/// before it run first, then the gx/gy backfill, then the rest, so the
/// indexes are built over filled columns.
const GRID_INDEXES_VERSION: i64 = 20260828000000;

/// A fixed list of migrations as a source for a partial Migrator.
#[derive(Debug)]
struct Migrations(Vec<Migration>);

impl MigrationSource<'static> for Migrations {
    fn resolve(
        self,
    ) -> BoxFuture<'static, Result<Vec<Migration>, BoxDynError>> {
        Box::pin(async move { Ok(self.0) })
    }
}

async fn open_db_inner(
    db_path: String,
    mut status: impl FnMut(&str, Option<f32>),
) -> Result<SqlitePool> {
    let opt = SqliteConnectOptions::from_str(&db_path)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        // migrations and index builds hold the write lock for seconds on a
        // large db; writers wait rather than fail
        .busy_timeout(std::time::Duration::from_secs(60))
        // Page-cache cap, applied to every pooled connection at open.
        // cache_size is per connection, so the worst-case memory is this
        // times the hot connections of every pool; keep it modest. The
        // previous foreground PRAGMA of 2 GiB landed on one arbitrary
        // pooled connection per transition and accumulated across
        // connections until iOS jetsammed the app while scrubbing.
        .pragma("cache_size", db_cache_size_pragma());
    let conn = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(DB_POOL_MAX_CONNECTIONS)
        .connect_with(opt)
        .await
        .context("connecting to database")?;
    status("migrating schema", None);
    let before = Migrations(
        MIGRATOR
            .iter()
            .filter(|m| m.version < GRID_INDEXES_VERSION)
            .cloned()
            .collect(),
    );
    let mut before = Migrator::new(before).await?;
    // a migrated database has versions this partial source lacks
    before.set_ignore_missing(true);
    before.run(&conn).await.context("running migrations")?;
    grid::backfill(&conn, |filled, total| {
        status(
            "computing grid coordinates",
            Some(filled as f32 / total.max(1) as f32),
        )
    })
    .await
    .context("backfilling grid coordinates")?;
    status("building indexes", None);
    MIGRATOR
        .run(&conn)
        .await
        .context("running index migrations")?;
    sqlx::query("PRAGMA secure_delete = on;")
        .execute(&conn)
        .await
        .context("setting secure delete on")
        .log_error_and_continue();
    Ok(conn)
}

/// Wait until every database on disk has finished opening. Tests call this
/// after init, since opens run in the background.
pub async fn wait_for_dbs() {
    loop {
        let opening = crate::app_state::get_derived_state(|s| {
            s.mounted_dbs_on_disk
                .values()
                .any(|st| matches!(st, DbStatus::Opening { .. }))
        });
        if !opening {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Checkpoint a database file over a plain connection, no migrations. Used
/// by the raw file export so it works whatever state the schema is in.
pub async fn checkpoint_db_file(db_path: &str) {
    let opt = match SqliteConnectOptions::from_str(db_path) {
        Ok(opt) => opt
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(60)),
        Err(e) => {
            tracing::error!("checkpointing {db_path}: {e}");
            return;
        }
    };
    let conn = match SqlitePool::connect_with(opt).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("checkpointing {db_path}: connecting: {e}");
            return;
        }
    };
    checkpoint_wal(&conn)
        .await
        .context("checkpointing for export")
        .log_error_and_continue();
    conn.close().await;
}

/// Connections per pool. Also the multiplier on the per-connection cache
/// cap when every connection runs hot, so keep the two in sync via
/// db_cache_size_pragma().
const DB_POOL_MAX_CONNECTIONS: u32 = 10;

/// Per-connection page-cache cap (negative = KiB), from a total budget of
/// RAM/16 split across a fully hot pool: ~50 MiB per connection on an
/// 8 GB phone, ~12 MiB on a 2 GB one, keeping the worst case well under
/// each device's per-process jetsam limit. Clamped to [2 MiB (the SQLite
/// default), 200 MiB].
fn db_cache_size_pragma() -> String {
    let budget = crate::map::map_data::physical_memory_bytes() / 16;
    let per_conn_kib = budget / DB_POOL_MAX_CONNECTIONS as u64 / 1024;
    format!("-{}", per_conn_kib.clamp(2_000, 200_000))
}

/// Free the page caches of the remaining open pools, for backgrounding: a
/// suspended app holding hot caches is an early jetsam candidate. The
/// cache belongs to each connection and sqlx can't address pooled
/// connections individually, so hold them all at once and shrink each;
/// caches refill on use after foregrounding, so there's no counterpart.
pub async fn shed_db_caches() {
    let dbs = AppState::global().dbs.lock().unwrap().clone();
    for (id, pool) in dbs.iter() {
        let mut held = Vec::new();
        for _ in 0..pool.size() {
            // an in-flight query may hold a connection past backgrounding;
            // shrink the ones we can get quickly rather than waiting
            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                pool.acquire(),
            )
            .await
            {
                Ok(Ok(conn)) => held.push(conn),
                _ => break,
            }
        }
        for conn in held.iter_mut() {
            sqlx::query("PRAGMA shrink_memory;")
                .execute(&mut **conn)
                .await
                .with_context(|| format!("shrinking a cache of database {id}"))
                .log_error_and_continue();
        }
    }
}
