//! Handles database access and modification.
//!
//! # Schema
//!
//! Current schema, for quick reference:
//!
//! (
//!     id          INTEGER NOT NULL PRIMARY KEY,
//!     lat         REAL    NOT NULL,
//!     lon         REAL    NOT NULL,
//!     accuracy    REAL    NOT NULL,
//!     speed       REAL    NOT NULL,
//!     course      REAL    NOT NULL,
//!     timestamp   INTEGER NOT NULL UNIQUE ON CONFLICT IGNORE
//! ) STRICT;
//!
//! With "STRICT" SQLite will ensure that we only insert the correct type into
//! each column. However, it is still possible to compare mismatched types in
//! our queries, so we must be careful there as well. When selecting rows in a
//! particular range, make sure to use the SQLite unixepoch() function and not
//! date() or datetime(), which return strings.
//!
//! We also use the unix epoch as the timestamp because comparisons are simpler.
//! sqlx/time format timestamps as "2023-03-15T05:15:56Z" by default but
//! SQLite's datetime() function will return "2023-03-15 05:15:56". This means
//! that date comparisons work fine, but anything requiring higher precision may
//! fail to compare correctly, since "T" always compares greater than " ". Using
//! seconds since the unix epoch is less error prone.
//!
//! # Interface
//!
//! As explained in the schema section, times stored in the database are stored
//! as their unix epoch. When returning data to callers, the LocationRow is
//! converted to a common::Location where the time is encoded as a
//! time::OffsetDateTime.
//!

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    FromRow, SqlitePool,
};

use crate::{app_state::AppState, core::debug};

// Embed our migrations from "migrations/" into our binary at compile time
static MIGRATOR: Migrator = sqlx::migrate!();

/// Struct representation of a Location row in the table
#[derive(Clone, FromRow, Debug)]
pub struct LocationRow {
    pub id: i64,
    pub timestamp: i64,
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
}

/// Initialized the shared database pool given its path.
/// Call this once at startup.
pub async fn init_db(db_path: String) -> Result<SqlitePool> {
    let opt = SqliteConnectOptions::from_str(&db_path)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let conn = SqlitePool::connect_with(opt).await?;
    MIGRATOR.run(&conn).await?;
    Ok(conn)
}

/// Get a handle for the database pool.
pub fn get_db_pool() -> SqlitePool {
    AppState::global().db.clone()
}

/// Checkpoint the database so all transactions in the WAL file are flushed to
/// the main database file.
pub async fn checkpoint_db() {
    let conn = get_db_pool();
    sqlx::query("PRAGMA wal_checkpoint(FULL);")
        .execute(&conn)
        .await
        .unwrap();
}

/// Log a location event in the database.
pub async fn log_location(
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    timestamp: i64,
) -> Result<()> {
    let conn = get_db_pool();
    sqlx::query!(
        "INSERT INTO location
            (lat, lon, accuracy, speed, course, timestamp)
        VALUES
            (?,?,?,?,?,?)",
        lat,
        lon,
        accuracy,
        speed,
        course,
        timestamp
    )
    .execute(&conn)
    .await?;
    Ok(())
}

/// Get the last record in the database
pub async fn get_last_record() -> Option<common::Location> {
    let conn = get_db_pool();

    // compile-time checked query macros are failing to infer the right type,
    // so we use the ordinary unchecked version instead for simplicity.
    /*
    let mut result = sqlx::query_as!(
    Location,
    r#"SELECT
    id as "id!",
    lat as "lat!",
    lon as "lon!",
    accuracy as "accuracy!",
    speed as "speed!",
    course as "course!",
    timestamp as "timestamp!: time::OffsetDateTime"
    FROM location
    ORDER BY timestamp
    DESC LIMIT 1"#,
    )
    */

    let mut result = sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_all(&conn)
    .await
    .unwrap();
    result.pop().map(|l| l.into())
}

pub async fn get_records_time_range(
    time_range: &common::TimeRange,
) -> Vec<common::Location> {
    let conn = get_db_pool();
    let result = sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location WHERE timestamp >= (?) AND timestamp <= (?)",
    )
    .bind(time_range.start.unix_timestamp())
    .bind(time_range.end.unix_timestamp())
    .fetch_all(&conn)
    .await
    .unwrap();
    // .into_iter() goes over the items, transferring ownership (.iter() would
    // give references)
    result.into_iter().map(|l| l.into()).collect()
}

/// Count the number of locations logged in the past hour.
///
/// in SQLite, can also do time operations like so:
///     WHERE timestamp >= unixepoch('now','-1 hour')"
pub async fn count_records_past_hour() -> i32 {
    let hour_ago = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
    count_records_since(hour_ago).await
}

pub async fn count_records_since(thresh: time::OffsetDateTime) -> i32 {
    let conn = get_db_pool();
    let timestamp = thresh.unix_timestamp();
    let result = sqlx::query!(
        "SELECT
            count(*) as count
        FROM location
        WHERE timestamp >= (?)",
        timestamp
    )
    .fetch_one(&conn)
    .await
    .unwrap();
    result.count
}

async fn count_all_records(conn: &SqlitePool) -> i32 {
    let result = sqlx::query!(
        "SELECT
            count(*) as count
        FROM location",
    )
    .fetch_one(conn)
    .await
    .unwrap();
    result.count
}

/// Import records from a database. The database must be writable so we can
/// migrate it to the current schema, if it's out of date.
pub async fn import_database_records(import_db_path: PathBuf) {
    let import_db_url = import_db_path.display().to_string();
    debug(&format!("importing file at {}", import_db_url));
    // Open a connection to the database if possible
    let import_conn = match SqlitePool::connect(&import_db_url).await {
        Ok(c) => c,
        Err(e) => {
            debug(&format!("Failed to open connection to import db. Err: {e}"));
            // TODO: send failure feedback to user
            return;
        }
    };
    // Migrate the databse and close it.
    if let Err(e) = MIGRATOR.run(&import_conn).await {
        debug(&format!("Failed to migrate import db. Err: {e}"));
        import_conn.close().await;
        return;
    }
    let n_to_import = count_all_records(&import_conn).await;
    import_conn.close().await;

    let conn = get_db_pool();
    let n_initial = count_all_records(&conn).await;

    let result = sqlx::query(&format!(
        "ATTACH '{}' as toMerge;
        BEGIN;
        INSERT OR IGNORE INTO location
            (lat, lon, accuracy, speed, course, timestamp)
        SELECT
            lat,lon,accuracy,speed,course,timestamp
        FROM toMerge.location;
        COMMIT;
        DETACH toMerge;",
        import_db_path.display()
    ))
    .execute(&conn)
    .await;
    if let Err(e) = result {
        debug(&format!("Failed to import records. Err: {e}"));
        return;
    }

    let n_final = count_all_records(&conn).await;
    let n_imported = n_final - n_initial;

    debug(&format!(
        "Successfully imported {} records. ({} duplicates ignored.)",
        n_imported,
        n_to_import - n_imported
    ));

    // TODO: send success to UI
}

/// Convert between the Location we have for talking to the database and the
/// Location we pass between the frontend and the backend.
///
/// We don't use the same type for each because we want the database version
/// to implement FromRow, and for common::Location to not implement it.
impl std::convert::From<LocationRow> for common::Location {
    fn from(loc: LocationRow) -> Self {
        common::Location {
            lat: loc.lat,
            lon: loc.lon,
            accuracy: loc.accuracy,
            speed: loc.speed,
            course: loc.course,
            datetime: time::OffsetDateTime::from_unix_timestamp(loc.timestamp)
                .unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{local::test_setup, paths::get_documents_dir};

    use super::*;

    #[tokio::test]
    async fn test_get_db_pool() {
        test_setup("test_get_db_pool/").await;
        get_db_pool();
    }

    #[tokio::test]
    async fn test_records_time_range() {
        test_setup("test_records_time_range/").await;

        // 5 and 10 seconds past the epoch
        log_location(1.0, 2.0, 3.0, 4.0, 5.0, 5).await.unwrap();
        log_location(0.0, 0.0, 0.0, 0.0, 0.0, 10).await.unwrap();
        let start = time::OffsetDateTime::from_unix_timestamp(3).unwrap();
        let end = time::OffsetDateTime::from_unix_timestamp(7).unwrap();
        // get the first
        let records =
            get_records_time_range(&common::TimeRange { start, end }).await;
        assert_eq!(records.len(), 1);
        // small integer floats can be exactly compared
        assert!(records[0].lat == 1.0);
        assert!(records[0].lon == 2.0);
    }

    /// Test that importing records into the database works as expected.
    #[tokio::test]
    async fn test_db_import() {
        test_setup("test_db_import/").await;

        log_location(1.0, 2.0, 3.0, 4.0, 5.0, 5).await.unwrap();
        log_location(0.0, 0.0, 0.0, 0.0, 0.0, 10).await.unwrap();

        let docdir = get_documents_dir();
        let db_name = "to_import.db";
        let db_path = docdir.join(db_name);
        let db_url = format!("sqlite://{}", db_path.display());
        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let import_conn = SqlitePool::connect_with(opt).await.unwrap();
        MIGRATOR.run(&import_conn).await.unwrap();

        sqlx::query(
            "INSERT INTO location
            (lat, lon, accuracy, speed, course, timestamp)
            VALUES
            (?,?,?,?,?,?)",
        )
        .bind(1.0)
        .bind(1.0)
        .bind(1.0)
        .bind(1.0)
        .bind(1.0)
        .bind(5) // duplicate timestamp
        .execute(&import_conn)
        .await
        .unwrap();

        // new data
        sqlx::query(
            "INSERT INTO location
            (lat, lon, accuracy, speed, course, timestamp)
            VALUES
            (?,?,?,?,?,?)",
        )
        .bind(2.0)
        .bind(2.0)
        .bind(2.0)
        .bind(2.0)
        .bind(2.0)
        .bind(15) // unique timestamp
        .execute(&import_conn)
        .await
        .unwrap();
        import_conn.close().await;

        // import records
        import_database_records(db_path).await;

        let start = time::OffsetDateTime::from_unix_timestamp(0).unwrap();
        let end = time::OffsetDateTime::from_unix_timestamp(20).unwrap();
        let records =
            get_records_time_range(&common::TimeRange { start, end }).await;

        assert_eq!(records.len(), 3);
        assert_eq!(
            records,
            vec![
                common::Location {
                    lat: 1.0,
                    lon: 2.0,
                    accuracy: 3.0,
                    speed: 4.0,
                    course: 5.0,
                    datetime: time::OffsetDateTime::from_unix_timestamp(5)
                        .unwrap(),
                },
                common::Location {
                    lat: 0.0,
                    lon: 0.0,
                    accuracy: 0.0,
                    speed: 0.0,
                    course: 0.0,
                    datetime: time::OffsetDateTime::from_unix_timestamp(10)
                        .unwrap(),
                },
                common::Location {
                    lat: 2.0,
                    lon: 2.0,
                    accuracy: 2.0,
                    speed: 2.0,
                    course: 2.0,
                    datetime: time::OffsetDateTime::from_unix_timestamp(15)
                        .unwrap(),
                },
            ]
        );
    }

    /// Test that migrating the database works, and that the data persists.
    #[tokio::test]
    async fn test_unique_timestamp_migration() {
        test_setup("test_unique_timestamp_migration/").await;

        let docdir = get_documents_dir();
        let db_name = "to_migrate.db";
        let db_url = format!("sqlite://{}", docdir.join(db_name).display());

        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let conn = SqlitePool::connect_with(opt.clone()).await.unwrap();

        let all_migrations = &MIGRATOR;
        println!("Have {} migrations", all_migrations.iter().len());
        let first_migration =
            MyMigration(all_migrations.iter().next().unwrap().clone());
        let first_migrator = sqlx::migrate::Migrator::new(&first_migration)
            .await
            .unwrap();

        first_migrator.run(&conn).await.unwrap();

        let data = LocationRow {
            id: 0,
            timestamp: 10,
            lat: 37.,
            lon: -122.,
            accuracy: 5.2,
            speed: 4.,
            course: 180.,
        };
        sqlx::query(
            "INSERT INTO location
        (lat, lon, accuracy, speed, course, timestamp)
        VALUES
        (?,?,?,?,?,?)",
        )
        .bind(data.lat)
        .bind(data.lon)
        .bind(data.accuracy)
        .bind(data.speed)
        .bind(data.course)
        .bind(data.timestamp)
        .execute(&conn)
        .await
        .unwrap();

        // show_migrations(&conn).await;
        all_migrations.run(&conn).await.unwrap();
        // show_migrations(&conn).await;

        // close and reopen the pool so we definitely get the new schema
        conn.close().await;
        let conn = SqlitePool::connect_with(opt).await.unwrap();

        // check we can get the data back out
        let retrieved = sqlx::query_as::<_, LocationRow>(
            "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_all(&conn)
        .await
        .unwrap();

        // compare with common::Location to ignore the id
        let res: common::Location = retrieved[0].clone().into();
        let exp: common::Location = data.into();
        assert_eq!(exp, res);
    }

    /// Debugging helpers for showing the state of the database migrations table
    #[allow(dead_code)]
    async fn show_migrations(conn: &SqlitePool) {
        let retrieved =
            sqlx::query_as::<_, MigrationRow>("SELECT * FROM _sqlx_migrations")
                .fetch_all(conn)
                .await
                .unwrap();
        println!("{:?}", &retrieved);
    }

    #[allow(dead_code)]
    #[derive(Clone, FromRow, Debug)]
    struct MigrationRow {
        pub version: i64,
        pub description: String,
        pub installed_on: time::PrimitiveDateTime,
        pub success: bool,
        pub checksum: Vec<u8>,
        pub execution_time: i64,
    }

    use futures_core::future::BoxFuture;
    use sqlx::error::BoxDynError;
    use sqlx::migrate::{Migration, MigrationSource};

    // Single migration that can be applied.
    #[derive(Debug, Clone)]
    struct MyMigration(Migration);

    impl<'s> MigrationSource<'s> for &'s MyMigration {
        fn resolve(self) -> BoxFuture<'s, Result<Vec<Migration>, BoxDynError>> {
            Box::pin(async move { Ok(vec![self.0.clone()]) })
        }
    }
}
