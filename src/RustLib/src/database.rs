//! Handles database access and modification.

use std::cell::RefCell;

use anyhow::{bail, Result};
use sqlx::types::time;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};

use crate::paths;
use crate::runtime;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL,
    lat         REAL                NOT NULL,
    lon         REAL                NOT NULL,
    accuracy    REAL                NOT NULL,
    speed       REAL                NOT NULL,
    course      REAL                NOT NULL,
    datetime    DATETIME            NOT NULL
);
";

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
thread_local!(static DB: RefCell<Option<SqlitePool>> = RefCell::new(None));

/// Initialized the shared database pool given its path.
/// Call this once at startup. Cannot be called within a tokio runtime since
/// one is used to do the intialization.
pub fn init_db() -> Result<()> {
    let db_path = paths::get_db_path()?;
    DB.with(|db| {
        let binding = runtime::get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            if !Sqlite::database_exists(&db_path).await? {
                Sqlite::create_database(&db_path).await?;
            }
            let conn = SqlitePool::connect(&db_path).await?;
            // ok to attempt to recreate table if it already exists
            sqlx::query(SCHEMA).execute(&conn).await?;
            let mut handle = db.borrow_mut();
            *handle = Some(conn.clone());
            Ok(())
        })
    })
}

/// Get a handle for the database pool.
pub async fn get_db_pool() -> Result<SqlitePool> {
    DB.with(|db| {
        let handle = db.borrow();
        match &*handle {
            Some(conn) => Ok(conn.clone()),
            None => bail!("Database not initialized."),
        }
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
) {
    let conn = match get_db_pool().await {
        Ok(c) => c,
        Err(_e) => return,
    };
    let datetime =
        time::OffsetDateTime::from_unix_timestamp(datetime_epoch).unwrap();
    let result = sqlx::query(
        "INSERT INTO location (lat, lon, accuracy, speed, course, datetime) VALUES (?,?,?,?,?,?)")
        .bind(lat)
        .bind(lon)
        .bind(accuracy)
        .bind(speed)
        .bind(course)
        .bind(datetime)
        .execute(&conn)
        .await;
    match result {
        // Ok(res) => println!("Query result: {:?}", res),
        Ok(_) => return,
        Err(e) => println!("Failed to log location due to error {}", e),
    };
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

    #[test]
    fn test_get_db_pool() {
        let binding = runtime::get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            // haven't set the storage path, so we expect an error
            assert!(get_db_pool().await.is_err());
        });

        test_setup("test_get_db_pool/");

        let binding = runtime::get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            assert!(get_db_pool().await.is_ok());
        });
    }

    #[test]
    fn test_log_location() {
        test_setup("test_log_location/");

        let binding = runtime::get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            log_location(
                1.0,
                2.0,
                3.0,
                4.0,
                5.0,
                time::OffsetDateTime::now_utc().unix_timestamp(),
            )
            .await;
            log_location(0.0, 0.0, 0.0, 0.0, 0.0, 0).await; // 1970
            let records = get_records_past_week().await;
            // old record should not appear
            assert_eq!(records.len(), 1);
            // small integer floats can be exactly compared
            assert!(records[0].lat == 1.0);
            assert!(records[0].lon == 2.0);
            assert!(records[0].accuracy == 3.0);
            assert!(records[0].speed == 4.0);
            assert!(records[0].course == 5.0);
        });
    }

    #[test]
    fn test_records_time_range() {
        test_setup("test_records_time_range/");

        let binding = runtime::get_runtime_binding();
        let rt = binding.borrow();
        rt.block_on(async {
            // 5 and 10 seconds past the epoch
            log_location(1.0, 2.0, 3.0, 4.0, 5.0, 5).await;
            log_location(0.0, 0.0, 0.0, 0.0, 0.0, 10).await;
            let records = get_records_time_range(3, 7).await; // get the first
            assert_eq!(records.len(), 1);
            // small integer floats can be exactly compared
            assert!(records[0].lat == 1.0);
            assert!(records[0].lon == 2.0);
            assert!(records[0].accuracy == 3.0);
            assert!(records[0].speed == 4.0);
            assert!(records[0].course == 5.0);
        });
    }
}
