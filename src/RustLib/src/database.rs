//! Handles database access and modification.

use std::error::Error;

use sqlx::sqlite::SqlitePool;

use crate::paths;

async fn get_db_pool() -> Result<SqlitePool, Box<dyn Error>> {
    Ok(SqlitePool::connect(&paths::get_db_path()?).await?)
}

#[cfg(test)]
mod tests {
    use super::get_db_pool;

    #[tokio::test]
    async fn test_get_db_pool() {
        assert!(get_db_pool().await.is_err());
    }
}

/// Log a location event in the database.
pub fn log_location(lat: f64, lon: f64, accuracy: f64, speed: f64, course: f64) {
    // arguments are received as 64-bit float, only lat/lon can't be
    // truncated to 32-bit without (potentially) losing some accuracy,
    // truncation should happen before storing in DB
    // TODO: figure out time
    println!(
        "{:?}, {:?}, {:?}, {:?}, {:?}",
        lat, lon, accuracy, speed, course
    );
}
