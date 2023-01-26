//! Manages the paths used to access files in the app.

use std::cell::RefCell;

static DB_FNAME: &str = "mydata.sqlite";

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
pub fn get_storage_dir() -> Result<String, &'static str> {
    CTX.with(|ctx| match &(*ctx.borrow()).storage_dir {
        Some(path) => Ok(path.clone()),
        None => Err("DB path not set."),
    })
}

/// Get the database path as a string.
pub fn get_db_path() -> Result<String, &'static str> {
    let mut storage_dir = get_storage_dir()?;
    storage_dir.push_str(DB_FNAME); // TODO: safe path appending
    Ok(storage_dir)
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
        let mut expected = dir;
        assert_eq!(get_storage_dir().unwrap(), expected);
        expected.push_str(DB_FNAME);
        assert_eq!(get_db_path().unwrap(), expected);
    }
}
