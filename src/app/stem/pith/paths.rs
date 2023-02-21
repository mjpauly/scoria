//! Manages the paths used to access files in the app.

use std::cell::RefCell;
use std::path::PathBuf;

use anyhow::{bail, Result};

const DB_PREFIX: &str = "sqlite://";
const DB_FNAME: &str = "data.db";

const VIZ_FNAME: &str = "viz.html";

// Our state data is stored in PATHS
// We don't know the contents during runtime initialization so it starts empty
thread_local!(static PATHS: RefCell<Paths> = RefCell::new(Paths::new_empty()));

// Context struct that contains the documents directory path.
// Lets us keep the directory path without having to pass it from Swift through
// every function call.
#[allow(dead_code)] // not yet using some directories
pub struct Paths {
    documents_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
    temp_dir: Option<PathBuf>,
    bundle_dir: Option<PathBuf>,
}

impl Paths {
    fn new_empty() -> Paths {
        Paths {
            documents_dir: None,
            library_dir: None,
            temp_dir: None,
            bundle_dir: None,
        }
    }
    pub fn new(
        documents_dir: Option<PathBuf>,
        library_dir: Option<PathBuf>,
        temp_dir: Option<PathBuf>,
        bundle_dir: Option<PathBuf>,
    ) -> Paths {
        Paths {
            documents_dir,
            library_dir,
            temp_dir,
            bundle_dir,
        }
    }
}

pub fn set_app_dirs(paths_to_set: Paths) {
    PATHS.with(|p| {
        *p.borrow_mut() = paths_to_set;
    });
}

/// Get the documents directory in the thread_local documents, returning an ownable
/// string.
pub fn get_documents_dir() -> Result<PathBuf> {
    let dir = PATHS.with(|p| (*p.borrow()).documents_dir.clone());
    match dir {
        Some(path) => Ok(path),
        None => bail!(
            "Documents directory not set! Call set_app_dirs() at startup."
        ),
    }
}

/// Get the database path as a string.
pub fn get_db_path() -> Result<String> {
    let documents_dir = get_documents_dir()?;
    let db_path = format!(
        "{}{}",
        DB_PREFIX,
        std::path::Path::new(&documents_dir)
            .join(DB_FNAME)
            .display()
    );
    Ok(db_path)
}

/// Get the path to the vizualization file.
pub fn get_viz_path() -> std::path::PathBuf {
    get_documents_dir().unwrap().join(VIZ_FNAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_setup;

    #[tokio::test]
    async fn test_set_paths() {
        assert!(get_documents_dir().is_err());
        assert!(get_db_path().is_err());
        let dir = String::from("./test_set_paths/");
        test_setup(&dir).await;
        assert_eq!(
            get_documents_dir().unwrap(),
            PathBuf::from("./test_set_paths/Documents")
        );
        let expected =
            format!("{}{}{}{}", DB_PREFIX, dir, "Documents/", DB_FNAME);
        assert_eq!(get_db_path().unwrap(), expected);
    }
}
