//! Handles database access and modification.

use std::error::Error;

use sqlx::types::time;
use sqlx::{migrate::MigrateDatabase, FromRow, Row, Sqlite, SqlitePool};

use crate::paths;

#[derive(Clone, FromRow, Debug)]
struct User {
    id: i64,
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
    datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}

const SQLITE_CANTOPEN: &str = "14"; // SQLite error code when it can't open DB

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

async fn create_db(db_path: &str) -> Result<SqlitePool, Box<dyn Error>> {
    if Sqlite::database_exists(&db_path).await.unwrap() {
        panic!("Trying the create database but it already exists")
    }
    Sqlite::create_database(db_path).await.unwrap();
    let conn = SqlitePool::connect(&db_path).await.unwrap();
    let result = sqlx::query(SCHEMA).execute(&conn).await.unwrap();
    // println!("Create table result: {:?}", result);
    Ok(conn)
}

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
    // TODO: figure out time
    let conn = get_db_pool().await.unwrap();
    let datetime =
        time::OffsetDateTime::from_unix_timestamp(datetime_epoch).unwrap();
    let result = sqlx::query("INSERT INTO location (lat, lon, accuracy, speed, course, datetime) VALUES (?,?,?,?,?,?)")
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

#[cfg(test)]
mod tests {
    use crate::paths;

    use super::*;

    #[tokio::test]
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

    #[tokio::test]
    async fn test_log_location() {
        paths::set_storage_dir(String::from("./"));
        log_location(0.0, 0.0, 0.0, 0.0, 0.0, 0).await;
    }
}
