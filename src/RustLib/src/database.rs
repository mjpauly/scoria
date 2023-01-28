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
    time: time::OffsetDateTime, // OffsetDateTime is timezone aware
}

static SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL,
    lat         REAL                NOT NULL,
    lon         REAL                NOT NULL,
    accuracy    REAL                NOT NULL,
    speed       REAL                NOT NULL,
    course      REAL                NOT NULL,
    time        DATETIME            NOT NULL
);
";

async fn create_db_if_missing() -> Result<(), Box<dyn Error>> {
    let db_path = paths::get_db_path()?;
    println!("in create db if missing");
    if !Sqlite::database_exists(&db_path).await.unwrap_or(false) {
        println!("Creating database {}", db_path);
        match Sqlite::create_database(&db_path).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("Database creation error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let conn = SqlitePool::connect(&db_path).await.unwrap();

    let result = sqlx::query(SCHEMA).execute(&conn).await.unwrap();
    println!("Create table result: {:?}", result);

    Ok(())
}

async fn get_db_pool() -> Result<SqlitePool, Box<dyn Error>> {
    create_db_if_missing().await?;
    let db_path = paths::get_db_path()?;
    Ok(SqlitePool::connect(&db_path).await?)
}

#[cfg(test)]
mod tests {
    use crate::paths;

    use super::get_db_pool;

    #[tokio::test]
    async fn test_get_db_pool() {
        // haven't set the storage path, so we expect an error
        assert!(get_db_pool().await.is_err());
        assert!(get_db_pool() // check that it is the error we expect
            .await
            .err()
            .unwrap()
            .is::<paths::StorageDirNotSetError>());
        paths::set_storage_dir(String::from("./"));
        // println!("{}", get_db_pool().await.err().unwrap());
        assert!(get_db_pool().await.is_ok());
    }
}

/// Log a location event in the database.
pub fn log_location(
    lat: f64,
    lon: f64,
    accuracy: f64,
    speed: f64,
    course: f64,
) {
    // arguments are received as 64-bit float, only lat/lon can't be
    // truncated to 32-bit without (potentially) losing some accuracy,
    // truncation should happen before storing in DB
    // TODO: figure out time
    println!(
        "{:?}, {:?}, {:?}, {:?}, {:?}",
        lat, lon, accuracy, speed, course
    );
}
