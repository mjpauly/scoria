//! Handles database access and modification.

use std::cell::RefCell;

use anyhow::{bail, Result};
use sqlx::types::time;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};

use crate::paths;

const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL,
    lat         REAL                NOT NULL,
    lon         REAL                NOT NULL,
    accuracy    REAL                NOT NULL,
    speed       REAL                NOT NULL,
    course      REAL                NOT NULL,
    datetime    DATETIME            NOT NULL
);";

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

// Shared database pool
// RefCell provides interior mutability so it can be initialized with the
// database handle when it is created. The Option is None until that happens.
thread_local!(static DB: RefCell<Option<SqlitePool>> = RefCell::new(None));

/// Initialized the shared database pool given its path.
/// Call this once at startup.
pub async fn init_db() -> Result<()> {
    let db_path = paths::get_db_path()?;
    if !Sqlite::database_exists(&db_path).await? {
        Sqlite::create_database(&db_path).await?;
    }
    let conn = SqlitePool::connect(&db_path).await?;
    sqlx::query(SCHEMA).execute(&conn).await?; // ok to attempt to recreate
                                               // table if it already exists
    DB.with(|db| {
        *db.borrow_mut() = Some(conn.clone());
    });
    Ok(())
}

/// Get a handle for the database pool.
pub async fn get_db_pool() -> Result<SqlitePool> {
    DB.with(|db| match &*db.borrow() {
        Some(conn) => Ok(conn.clone()),
        None => bail!("Database not initialized."),
    })
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
    let conn = get_db_pool().await?;
    let datetime = time::OffsetDateTime::from_unix_timestamp(datetime_epoch)?;
    sqlx::query(
        "INSERT INTO location (lat, lon, accuracy, speed, course, datetime)
VALUES (?,?,?,?,?,?)",
    )
    .bind(lat)
    .bind(lon)
    .bind(accuracy)
    .bind(speed)
    .bind(course)
    .bind(datetime)
    .execute(&conn)
    .await?;
    Ok(())
}

/// Retrieve a vector of locations from the past week.
pub async fn get_records_past_week() -> Vec<Location> {
    let conn = get_db_pool().await.unwrap();
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
    let conn = get_db_pool().await.unwrap();
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

#[cfg(test)]
mod tests {
    use crate::tests::test_setup;

    use super::*;

    #[tokio::test]
    async fn test_get_db_pool() {
        // haven't set the storage path, so we expect an error
        assert!(get_db_pool().await.is_err());

        test_setup("test_get_db_pool/").await;

        assert!(get_db_pool().await.is_ok());
    }

    #[tokio::test]
    async fn test_log_location() {
        test_setup("test_log_location/").await;

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
        test_setup("test_records_time_range/").await;

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
