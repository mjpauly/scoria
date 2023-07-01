//! Handles database access and modification.
//!
//! # Schema
//!
//! Current schema, for quick reference:
//!
//! CREATE TABLE tmplocation
//! (
//!     id                          INTEGER NOT NULL PRIMARY KEY,
//!     timestamp                   INTEGER NOT NULL UNIQUE ON CONFLICT IGNORE,
//!
//!     latitude                    REAL    NOT NULL,
//!     longitude                   REAL    NOT NULL,
//!     horizontal_accuracy         REAL    NOT NULL,
//!
//!     msl_altitude                REAL,
//!     ellipsoid_altitude          REAL,
//!     vertical_accuracy           REAL,
//!     story                       INTEGER,
//!
//!     speed                       REAL,
//!     speed_accuracy              REAL,
//!
//!     course                      REAL,
//!     course_accuracy             REAL,
//!
//!     is_simulated_by_software    INTEGER,
//!     is_produced_by_accessory    INTEGER,
//!     was_imported                INTEGER NOT NULL DEFAULT 0
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

use crate::{
    app_state::AppState,
    core::{debug, error},
};

// Embed our migrations from "migrations/" into our binary at compile time
static MIGRATOR: Migrator = sqlx::migrate!();

/// Struct representation of a Location row in the table
#[derive(Clone, FromRow, Debug)]
pub struct LocationRow {
    pub id: i64,
    pub timestamp: i64,

    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,

    pub msl_altitude: Option<f64>,
    pub ellipsoid_altitude: Option<f64>,
    pub vertical_accuracy: Option<f64>,
    pub story: Option<i64>,

    pub speed: Option<f64>,
    pub speed_accuracy: Option<f64>,

    pub course: Option<f64>,
    pub course_accuracy: Option<f64>,

    pub is_simulated_by_software: Option<bool>,
    pub is_produced_by_accessory: Option<bool>,

    pub was_imported: bool,
}

/// C FFI struct definition with special ways of encoding unavailable data
#[repr(C)]
#[derive(Clone)]
pub struct OSLocationData {
    // always available fields
    pub timestamp: i64,

    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,

    pub msl_altitude: f64,
    pub ellipsoid_altitude: f64,
    pub vertical_accuracy: f64,

    // bool indivates if story data is available
    pub story_available: bool,
    pub story: i64,

    // marked as unavailable with -1
    pub speed: f64,
    pub speed_accuracy: f64,
    pub course: f64,
    pub course_accuracy: f64,

    // indicates if source info is available, or should be NULL
    pub source_info_available: bool,
    pub is_simulated_by_software: bool,
    pub is_produced_by_accessory: bool,
}

/// common::Location is about the representation needed for inserting into the
/// database, so we use this parser for log_location()
impl std::convert::From<OSLocationData> for common::Location {
    fn from(loc: OSLocationData) -> Self {
        let some_if_geq_zero = |val| (val >= 0.0).then_some(val);
        Self {
            timestamp: time::OffsetDateTime::from_unix_timestamp(loc.timestamp)
                .unwrap(),
            latitude: loc.latitude,
            longitude: loc.longitude,
            horizontal_accuracy: loc.horizontal_accuracy,
            msl_altitude: Some(loc.msl_altitude),
            ellipsoid_altitude: Some(loc.ellipsoid_altitude),
            vertical_accuracy: some_if_geq_zero(loc.vertical_accuracy),
            story: loc.story_available.then_some(loc.story),
            speed: some_if_geq_zero(loc.speed),
            speed_accuracy: some_if_geq_zero(loc.speed_accuracy),
            course: some_if_geq_zero(loc.course),
            course_accuracy: some_if_geq_zero(loc.course_accuracy),
            is_simulated_by_software: loc
                .source_info_available
                .then_some(loc.is_simulated_by_software),
            is_produced_by_accessory: loc
                .source_info_available
                .then_some(loc.is_produced_by_accessory),
            // data comes from OS -> mark as not imported
            was_imported: false,
        }
    }
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
/// the main database file. Also vacuum the database to reclaim unused pages and
/// save space. Vacuuming happens first since it behaves like a normal
/// transaction, then checkpointing puts all outstanding transactions into the
/// main database file.
///
/// This should be done anytime the user wants to export the database file, or
/// during app startup since migrations can cause large amounts of space to be
/// unused if they involve copying data to a new table.
pub async fn checkpoint_db() {
    let conn = get_db_pool();
    if let Err(e) = sqlx::query("VACUUM; PRAGMA wal_checkpoint(FULL);")
        .execute(&conn)
        .await
    {
        error("Failed to checkpoint/vacuum db.", e);
        // not a fatal error, continue onwards
    }
}

/// Log a location event in the database.
pub async fn log_location(loc: OSLocationData) -> Result<()> {
    // Convert to a common::Location, which has the correct fields
    let parsed: common::Location = loc.into();
    let conn = get_db_pool();
    let timestamp = parsed.timestamp.unix_timestamp();
    sqlx::query!(
        "INSERT INTO location (
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory
        )
        VALUES
            (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        timestamp,
        parsed.latitude,
        parsed.longitude,
        parsed.horizontal_accuracy,
        parsed.msl_altitude,
        parsed.ellipsoid_altitude,
        parsed.vertical_accuracy,
        parsed.story,
        parsed.speed,
        parsed.speed_accuracy,
        parsed.course,
        parsed.course_accuracy,
        parsed.is_simulated_by_software,
        parsed.is_produced_by_accessory,
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

    match sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_all(&conn)
    .await
    {
        Ok(mut result) => result.pop().map(|l| l.into()),
        Err(e) => {
            error("Failed to get last record.", e);
            None
        }
    }
}

pub async fn get_records_time_range(
    time_range: &common::TimeRange,
) -> Vec<common::Location> {
    let conn = get_db_pool();
    match sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location WHERE timestamp >= (?) AND timestamp <= (?)",
    )
    .bind(time_range.start.unix_timestamp())
    .bind(time_range.end.unix_timestamp())
    .fetch_all(&conn)
    .await
    {
        Ok(result) => {
            // .into_iter() goes over the items, transferring ownership
            // (.iter() would give references)
            result.into_iter().map(|l| l.into()).collect()
        }
        Err(e) => {
            error("Failed to get records in time range.", e);
            vec![]
        }
    }
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
    match sqlx::query!(
        "SELECT
            count(*) as count
        FROM location
        WHERE timestamp >= (?)",
        timestamp
    )
    .fetch_one(&conn)
    .await
    {
        Ok(result) => result.count,
        Err(e) => {
            error("Failed to count records since.", e);
            0
        }
    }
}

async fn count_all_records(conn: &SqlitePool) -> i32 {
    match sqlx::query!(
        "SELECT
            count(*) as count
        FROM location",
    )
    .fetch_one(conn)
    .await
    {
        Ok(result) => result.count,
        Err(e) => {
            error("Failed to count all records.", e);
            0
        }
    }
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
            error("Failed to open connection to import db.", e);
            // TODO: send failure feedback to user
            return;
        }
    };
    // Migrate the databse, mark all rows with was_imported=true, and close it.
    if let Err(e) = MIGRATOR.run(&import_conn).await {
        error("Failed to migrate import db.", e);
        import_conn.close().await;
        return;
    }
    if let Err(e) = sqlx::query!("UPDATE location SET was_imported = 1")
        .execute(&import_conn)
        .await
    {
        error("Failed to mark records as imported.", e);
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
        INSERT OR IGNORE INTO location (
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory,
            was_imported
        )
        SELECT
            timestamp,
            latitude, longitude, horizontal_accuracy,
            msl_altitude, ellipsoid_altitude,
            vertical_accuracy,
            story,
            speed, speed_accuracy,
            course, course_accuracy,
            is_simulated_by_software, is_produced_by_accessory,
            was_imported
        FROM toMerge.location;
        COMMIT;
        DETACH toMerge;",
        import_db_path.display()
    ))
    .execute(&conn)
    .await;
    if let Err(e) = result {
        error("Failed to import records.", e);
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
            timestamp: time::OffsetDateTime::from_unix_timestamp(loc.timestamp)
                .unwrap(),
            latitude: loc.latitude,
            longitude: loc.longitude,
            horizontal_accuracy: loc.horizontal_accuracy,
            msl_altitude: loc.msl_altitude,
            ellipsoid_altitude: loc.ellipsoid_altitude,
            vertical_accuracy: loc.vertical_accuracy,
            story: loc.story,
            speed: loc.speed,
            speed_accuracy: loc.speed_accuracy,
            course: loc.course,
            course_accuracy: loc.course_accuracy,
            is_simulated_by_software: loc.is_simulated_by_software,
            is_produced_by_accessory: loc.is_produced_by_accessory,
            was_imported: loc.was_imported,
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

    /// Generate location data that is different for each idx
    fn get_test_data(idx: usize) -> OSLocationData {
        OSLocationData {
            timestamp: idx as i64 * 5,
            latitude: idx as f64,
            longitude: idx as f64,
            horizontal_accuracy: idx as f64,
            msl_altitude: idx as f64,
            ellipsoid_altitude: idx as f64,
            vertical_accuracy: idx as f64,
            story_available: true,
            story: idx as i64,
            speed: idx as f64,
            speed_accuracy: idx as f64,
            course: idx as f64,
            course_accuracy: idx as f64,
            source_info_available: true,
            is_simulated_by_software: true,
            is_produced_by_accessory: false,
        }
    }

    #[tokio::test]
    async fn test_records_time_range() {
        test_setup("test_records_time_range/").await;

        // 5 and 10 seconds past the epoch
        log_location(get_test_data(1)).await.unwrap(); // timestamp: 5
        log_location(get_test_data(2)).await.unwrap(); // timestamp: 10
        let start = time::OffsetDateTime::from_unix_timestamp(3).unwrap();
        let end = time::OffsetDateTime::from_unix_timestamp(7).unwrap();
        // get the first
        let records =
            get_records_time_range(&common::TimeRange { start, end }).await;
        assert_eq!(records.len(), 1);
        // small integer floats can be exactly compared
        assert!(records[0].latitude == 1.0);
        assert!(records[0].longitude == 1.0);
    }

    /// Test that importing records into the database works as expected.
    #[tokio::test]
    async fn test_db_import() {
        test_setup("test_db_import/").await;

        // log some data in our main database
        log_location(get_test_data(1)).await.unwrap(); // timestamp: 5
        log_location(get_test_data(2)).await.unwrap(); // timestamp: 10

        // create a new database with a new record and a duplicate
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
            "INSERT INTO location (
                timestamp,
                latitude, longitude, horizontal_accuracy
            )
            VALUES
                (?,?,?,?)",
        )
        .bind(5) // duplicate timestamp
        .bind(-1.0)
        .bind(-1.0)
        .bind(1.0)
        .execute(&import_conn)
        .await
        .unwrap();

        // new data
        sqlx::query(
            "INSERT INTO location (
                timestamp,
                latitude, longitude, horizontal_accuracy
            )
            VALUES
                (?,?,?,?)",
        )
        .bind(15) // unique timestamp
        .bind(4.0)
        .bind(4.0)
        .bind(4.0)
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
                get_test_data(1).into(),
                get_test_data(2).into(),
                common::Location {
                    timestamp: time::OffsetDateTime::from_unix_timestamp(15)
                        .unwrap(),
                    latitude: 4.0,
                    longitude: 4.0,
                    horizontal_accuracy: 4.0,
                    msl_altitude: None,
                    ellipsoid_altitude: None,
                    vertical_accuracy: None,
                    story: None,
                    speed: None,
                    speed_accuracy: None,
                    course: None,
                    course_accuracy: None,
                    is_simulated_by_software: None,
                    is_produced_by_accessory: None,
                    was_imported: true,
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
            latitude: 37.,
            longitude: -122.,
            horizontal_accuracy: 5.2,
            msl_altitude: None,
            ellipsoid_altitude: None,
            vertical_accuracy: None,
            story: None,
            speed: Some(4.),
            speed_accuracy: None,
            course: Some(180.),
            course_accuracy: None,
            is_simulated_by_software: None,
            is_produced_by_accessory: None,
            was_imported: false,
        };
        sqlx::query(
            "INSERT INTO location
        (lat, lon, accuracy, speed, course, timestamp)
        VALUES
        (?,?,?,?,?,?)",
        )
        .bind(data.latitude)
        .bind(data.longitude)
        .bind(data.horizontal_accuracy)
        .bind(data.speed.unwrap())
        .bind(data.course.unwrap())
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

    /// # Test how costly nulls really are
    ///
    /// Primary finding from this test: Use the vacuum command to rebuild the
    /// database and shed unused pages, especially after migrations that involve
    /// copying all the data to a new table.
    ///
    /// Expected size of original: 7 * 8 * 100_000 = 5.6 MB
    /// Expected size of final: (7 * 8 + 9) * 100_000 = 6.5 MB
    /// Expected growth rate: 16%
    ///
    /// Observed size of original: 6.3 MB
    /// Observed size of final: 8.76 MB
    /// Observed growth rate: 39%
    ///
    /// Not sure why it's so much larger in the test.. The real data matches
    /// expected calculations much better.
    ///
    /// # Findings from real data (put here since it's the same subject area)
    ///
    /// Examining migration #2 (unique_timestamp) -> #4 (was_imported) on real
    /// data exported from the app.
    ///
    /// Results on 6/27 export, which was up to the unique_timestamp migration.
    ///         0. Original: 9.3 MB
    ///         1. Vacuumed: 9.1 MB
    ///         2. Vacuumed and then migrated: 19.5 MB
    ///         3. Vacuumed, migrated, and vacuumed: 10.2 MB
    /// 1 -> 3 should accurately show the null cost. Observed growth: 12%
    /// Calculation:
    ///         - We had 7 required columns before, 8 bytes each
    ///         - After we have 8 new nullable columns, but previously invalid
    ///                 speed and course data is now marked as NULL
    ///         - There were 132400 total records, 10170 with NULL speed and
    ///                 19307 with NULL course data
    ///         - Expected size of original: 7 * 8 * 132400 = 7.41 MB
    ///         - Expected size of final:
    ///                 (7 * 8 + 9 * 1) * 132400 - 10170 * 7 - 19307 * 7
    ///                 = 8.40 MB
    ///         - Expected growth: 13%
    ///         - Wow it actually matches to about 1%. Yay!
    ///
    /// With the new columns, here is the expected increase in each row's
    /// storage, assuming high accuracy GPS data:
    ///         With high accuracy GPS data:
    ///                 - before: 7 * 8 = 56 B
    ///                 - after: 12 * 8 + 4 = 100 B
    ///                 - increase in storage use: 79%
    ///         With low accuracy GPS data (no speed, course, vertical_accuracy)
    ///                 - after: 7 * 8 + 9 = 65 B
    ///                 - increase in storage use: 16%
    ///
    /// Expected storage usage rate:
    ///         - 10 MB * 1.79 / 3 months
    ///                 = 71 MB / year
    ///                 = 710 MB / decade (what! actually not very much)
    ///         - Of course, this depends on movement pattern. Someone who moves
    ///                 for a living (e.g. delivery driver), would have a
    ///                 storage growth rate that is probably 10x this or more.
    // #[tokio::test] // uncomment to run the test
    #[allow(dead_code)]
    async fn test_null_size() {
        test_setup("test_null_size/").await;

        let docdir = get_documents_dir();
        let db_name = "to_migrate.db";
        let db_path = docdir.join(db_name);
        let db_url = format!("sqlite://{}", db_path.display());

        let opt = SqliteConnectOptions::from_str(&db_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let conn = SqlitePool::connect_with(opt.clone()).await.unwrap();

        println!("Run dir: {}", std::env::current_dir().unwrap().display());
        let all_migrations = &MIGRATOR;
        println!("Have {} migrations", all_migrations.iter().len());
        let first_migration =
            MyMigration(all_migrations.iter().next().unwrap().clone());
        let first_migrator = sqlx::migrate::Migrator::new(&first_migration)
            .await
            .unwrap();

        first_migrator.run(&conn).await.unwrap();

        for _ in 0..100000 {
            sqlx::query(
                "INSERT INTO location
            (lat, lon, accuracy, speed, course, timestamp)
            VALUES
            (?,?,?,?,?,?)",
            )
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<f64>())
            .bind(rand::random::<i64>())
            .execute(&conn)
            .await
            .unwrap();
        }
        let get_size = || async {
            sqlx::query("PRAGMA wal_checkpoint(FULL); VACUUM;")
                .execute(&conn)
                .await
                .unwrap();
            let f =
                std::fs::File::open(format!("{}", db_path.display())).unwrap();
            println!("size: {}", f.metadata().unwrap().len());
        };
        get_size().await;

        // show_migrations(&conn).await;
        all_migrations.run(&conn).await.unwrap();
        // show_migrations(&conn).await;

        get_size().await;

        conn.close().await;
        assert_eq!(1, 0); // fail the test deliberately
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
