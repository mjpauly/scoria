//! Handles database access and modification.

use std::error::Error;

use sqlx::types::time;
use sqlx::{migrate::MigrateDatabase, FromRow, Row, Sqlite, SqlitePool};

use crate::paths;

// SQLite error code when it can't open DB
// https://www.sqlite.org/rescode.html#cantopen
const SQLITE_CANTOPEN: &str = "14";

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
    id: i64,
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}

/// Create a SQLite database with the given schema if one doesn't exist.
/// Panics if it already exists.
async fn create_db(db_path: &str) -> Result<SqlitePool, Box<dyn Error>> {
    if Sqlite::database_exists(&db_path).await.unwrap() {
        panic!("Trying the create database but it already exists")
    }
    Sqlite::create_database(db_path).await.unwrap();
    let conn = SqlitePool::connect(&db_path).await.unwrap();
    let result = sqlx::query(SCHEMA).execute(&conn).await.unwrap();
    println!("Create table result: {:?}", result);
    Ok(conn)
}

/// Get a SqlitePool connection to the database, which can process queries.
async fn get_db_pool() -> Result<SqlitePool, Box<dyn Error>> {
    let db_path = paths::get_db_path()?;
    let result = SqlitePool::connect(&db_path).await;
    match result {
        Ok(pool) => Ok(pool),
        Err(sqlx::error::Error::Database(source)) => {
            // database driver error
            match &*source
                .code() // get SQLite error code from Option<Cow<str>>
                .unwrap_or_else(|| std::borrow::Cow::Borrowed(""))
            {
                // if can't open it, we probably need to create it
                SQLITE_CANTOPEN => Ok(create_db(&db_path).await?),
                _ => panic!("Unkown database driver error: {}", source),
            }
        }
        Err(e) => panic!("Unkown sqlx error {}", e),
    }
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
    let conn = get_db_pool().await.unwrap();
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
        .await
        .unwrap();
    println!("Query result: {:?}", result);
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

#[cfg(test)]
mod tests {
    use crate::paths;

    use super::*;

    // Need a top level test so we don't get a race condition to create the
    // database, since each test would normally run in its own thread, but they
    // share the same working directory.
    #[tokio::test]
    async fn test_db_top() {
        test_get_db_pool().await;
        test_log_location().await;
    }

    async fn test_get_db_pool() {
        // haven't set the storage path, so we expect an error
        assert!(get_db_pool().await.is_err());
        assert!(get_db_pool() // check that it is the error we expect
            .await
            .err()
            .unwrap()
            .is::<paths::PathError>());
        paths::set_storage_dir(String::from("./"));
        assert!(get_db_pool().await.is_ok());
    }

    async fn test_log_location() {
        paths::set_storage_dir(String::from("./"));
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
    }
}
