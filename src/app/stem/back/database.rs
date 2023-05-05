//! Handles database access and modification.
//!
//! # Schema
//!
//! Current schema, for quick reference:
//!
//! CREATE TABLE IF NOT EXISTS location
//! (
//!     id          INTEGER PRIMARY KEY NOT NULL CHECK (typeof(id) = 'integer'),
//!     lat         REAL                NOT NULL CHECK (typeof(lat) = 'real'),
//!     lon         REAL                NOT NULL CHECK (typeof(lon) = 'real'),
//!     accuracy    REAL                NOT NULL CHECK (typeof(accuracy) = 'real'),
//!     speed       REAL                NOT NULL CHECK (typeof(speed) = 'real'),
//!     course      REAL                NOT NULL CHECK (typeof(course) = 'real'),
//!     timestamp   INTEGER             NOT NULL CHECK (typeof(timestamp) = 'integer')
//! );
//!
//! With the checks SQLite will ensure that we only insert the correct type into
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

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};

use crate::app_state::AppState;

/// Struct representation of a Location row in the table
#[derive(Clone, FromRow, Debug)]
pub struct LocationRow {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
    pub timestamp: i64,
}

/// Initialized the shared database pool given its path.
/// Call this once at startup.
pub async fn init_db(db_path: String) -> Result<SqlitePool> {
    if !Sqlite::database_exists(&db_path).await? {
        Sqlite::create_database(&db_path).await?;
    }
    let conn = SqlitePool::connect(&db_path).await?;
    // Embed our migrations from "migrations/" into our binary at compile time,
    // and migrate the database at runtime.
    sqlx::migrate!().run(&conn).await?;
    Ok(conn)
}

/// Get a handle for the database pool.
pub fn get_db_pool() -> SqlitePool {
    AppState::global().db.clone()
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
    use crate::local::test_setup;

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
}
