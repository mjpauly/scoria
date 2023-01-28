//! Manages the paths used to access files in the app.

use std::cell::RefCell;
use std::error::Error;
use std::fmt;

static DB_PREFIX: &str = "sqlite://";
static DB_FNAME: &str = "data.db";

// Our state data is stored in CTX
// We don't know the contents during runtime initialization so it starts empty
thread_local!(static CTX: RefCell<Ctx> = RefCell::new(
        Ctx { storage_dir: None }));

// Context struct that contains the storage directory path.
// Lets us keep the directory path without having to pass it from Swift through
// every function call.
struct Ctx {
    storage_dir: Option<String>,
}

/// Error type returned when the storage directory has not been updated yet.
#[derive(Debug, Clone)]
pub struct StorageDirNotSetError;
impl Error for StorageDirNotSetError {}
impl fmt::Display for StorageDirNotSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Storage directory not set! Call set_storage_dir() at startup."
        )
    }
}

/// Sets the storage directory in the thread_local storage, consuming the
/// string.
pub fn set_storage_dir(dir: String) {
    CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        println!("Set storage path to: {}", dir);
        ctx.storage_dir = Some(dir);
    });
}

/// Get the storage directory in the thread_local storage, returning an ownable
/// string.
pub fn get_storage_dir() -> Result<String, StorageDirNotSetError> {
    let dir = CTX.with(|ctx| (*ctx.borrow()).storage_dir.clone());
    match dir {
        Some(path) => Ok(path),
        None => Err(StorageDirNotSetError),
    }
}

/// Get the database path as a string.
pub fn get_db_path() -> Result<String, StorageDirNotSetError> {
    let storage_dir = get_storage_dir()?;
    // let db_path = format!("{}{}{}", DB_PREFIX, storage_dir, DB_FNAME);
    let db_path = format!(
        "{}{}",
        DB_PREFIX,
        std::path::Path::new(&storage_dir).join(DB_FNAME).display()
    );
    // TODO: safe path appending
    Ok(db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_dir_update() {
        assert!(get_storage_dir().is_err());
        assert!(get_db_path().is_err());
        let dir = String::from("./");
        set_storage_dir(dir.clone());
        assert_eq!(get_storage_dir().unwrap(), dir);
        let expected = format!("{}{}{}", DB_PREFIX, dir, DB_FNAME);
        assert_eq!(get_db_path().unwrap(), expected);
    }
}
