//! Handles database access and modification.
//!
//! Current schema, for quick reference:
//!
//! CREATE TABLE IF NOT EXISTS location
//! (
//!     id          INTEGER PRIMARY KEY NOT NULL,
//!     lat         REAL                NOT NULL,
//!     lon         REAL                NOT NULL,
//!     accuracy    REAL                NOT NULL,
//!     speed       REAL                NOT NULL,
//!     course      REAL                NOT NULL,
//!     datetime    DATETIME            NOT NULL
//! );";
//!

// use std::cell::RefCell;

use anyhow::Result;
use sqlx::types::time;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};

use crate::app_state::AppState;
use crate::common;

/// Struct representation of a Location row in the table
#[derive(Clone, FromRow, Debug)]
pub struct Location {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
    pub datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}

/// Initialized the shared database pool given its path.
/// Call this once at startup.
pub async fn init_db(db_path: String) -> Result<SqlitePool> {
    if !Sqlite::database_exists(&db_path).await? {
        Sqlite::create_database(&db_path).await?;
    }
    let conn = SqlitePool::connect(&db_path).await?;
    // ok to attempt to recreate table if it already exists
    // sqlx::query(SCHEMA).execute(&conn).await?;
    // embed our migrations from "migrations/" into our binary
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
    datetime_epoch: i64,
) -> Result<()> {
    let conn = get_db_pool();
    let datetime = time::OffsetDateTime::from_unix_timestamp(datetime_epoch)?;
    sqlx::query!(
        "INSERT INTO location (lat, lon, accuracy, speed, course, datetime)
VALUES (?,?,?,?,?,?)",
        lat,
        lon,
        accuracy,
        speed,
        course,
        datetime
    )
    .execute(&conn)
    .await?;
    Ok(())
}

/// Get the last record in the database
pub async fn get_last_record() -> Option<Location> {
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
    datetime as "datetime!: time::OffsetDateTime"
    FROM location
    ORDER BY datetime
    DESC LIMIT 1"#,
    )
    */

    let mut result = sqlx::query_as::<_, Location>(
        "SELECT * FROM location ORDER BY datetime DESC LIMIT 1",
    )
    .fetch_all(&conn)
    .await
    .unwrap();
    result.pop()
}

/// Retrieve a vector of locations from the past week.
pub async fn get_records_past_week() -> Vec<Location> {
    let conn = get_db_pool();
    let result = sqlx::query_as::<_, Location>(
        "SELECT * FROM location WHERE datetime >= date('now','-7 days')",
    )
    .fetch_all(&conn)
    .await
    .unwrap();
    result
}

pub async fn get_records_time_range(
    start_epoch: i64,
    end_epoch: i64,
) -> Vec<Location> {
    let conn = get_db_pool();
    let result = sqlx::query_as::<_, Location>(
        "SELECT * FROM location WHERE datetime >= (?) AND datetime <= (?)",
    )
    .bind(time::OffsetDateTime::from_unix_timestamp(start_epoch).unwrap())
    .bind(time::OffsetDateTime::from_unix_timestamp(end_epoch).unwrap())
    .fetch_all(&conn)
    .await
    .unwrap();
    result
}

pub async fn count_records_past_hour() -> i32 {
    let conn = get_db_pool();
    let result = sqlx::query!(
        "SELECT
            count(*) as count
        FROM location
        WHERE datetime >= date('now','-1 hours')"
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
impl std::convert::From<Location> for common::Location {
    fn from(loc: Location) -> Self {
        common::Location {
            lat: loc.lat,
            lon: loc.lon,
            accuracy: loc.accuracy,
            speed: loc.speed,
            course: loc.course,
            datetime: loc.datetime,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_setup_clean;

    use super::*;

    #[tokio::test]
    async fn test_get_db_pool() {
        // haven't set the storage path, so we expect an error
        // assert!(get_db_pool().await.is_err());

        test_setup_clean("test_get_db_pool/").await;

        // assert!(get_db_pool().await.is_ok());
        get_db_pool();
    }

    #[tokio::test]
    async fn test_log_location() {
        test_setup_clean("test_log_location/").await;

        let now = time::OffsetDateTime::now_utc();
        log_location(1.0, 2.0, 3.0, 4.0, 5.0, now.unix_timestamp())
            .await
            .unwrap();
        log_location(0.0, 0.0, 0.0, 0.0, 0.0, 0).await.unwrap(); // 1970
        let records = get_records_past_week().await;

        // old record should not appear
        assert_eq!(records.len(), 1);
        // small integer floats can be exactly compared
        assert!(records[0].lat == 1.0);
        assert!(records[0].lon == 2.0);
        assert!(records[0].accuracy == 3.0);
        assert!(records[0].speed == 4.0);
        assert!(records[0].course == 5.0);
    }

    #[tokio::test]
    async fn test_records_time_range() {
        test_setup_clean("test_records_time_range/").await;

        // 5 and 10 seconds past the epoch
        log_location(1.0, 2.0, 3.0, 4.0, 5.0, 5).await.unwrap();
        log_location(0.0, 0.0, 0.0, 0.0, 0.0, 10).await.unwrap();
        let records = get_records_time_range(3, 7).await; // get the first
        assert_eq!(records.len(), 1);
        // small integer floats can be exactly compared
        assert!(records[0].lat == 1.0);
        assert!(records[0].lon == 2.0);
    }
}
