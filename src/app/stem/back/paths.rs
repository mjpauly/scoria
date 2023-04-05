//! Manages the paths used to access files in the app.

use std::path::PathBuf;

use crate::app_state::AppState;

const DB_PREFIX: &str = "sqlite://";
const DB_FNAME: &str = "data.db";

// Struct that contains the app directory paths.
// Lets us keep the paths without having to pass it from Swift every function
// call.
#[allow(dead_code)] // not yet using some directories
#[derive(Debug, Clone)]
pub struct Paths {
    pub documents_dir: PathBuf,
    pub library_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub bundle_dir: PathBuf,
}

/// Get the documents directory in the app state, returning an ownable PathBuf.
pub fn get_documents_dir() -> PathBuf {
    AppState::global().paths.documents_dir.clone()
}

pub fn get_library_dir() -> PathBuf {
    AppState::global().paths.library_dir.clone()
}

pub fn get_bundle_dir() -> PathBuf {
    AppState::global().paths.bundle_dir.clone()
}

/// Get the database path as a string.
pub fn get_db_path() -> String {
    let documents_dir = get_documents_dir();
    get_db_path_helper(documents_dir)
}

pub fn get_db_path_helper(documents_dir: PathBuf) -> String {
    let db_path = format!(
        "{}{}",
        DB_PREFIX,
        std::path::Path::new(&documents_dir)
            .join(DB_FNAME)
            .display()
    );
    db_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::test_setup;

    #[tokio::test]
    async fn test_set_paths() {
        let dir = String::from("./test_set_paths/");
        test_setup(&dir).await;
        assert_eq!(
            get_documents_dir(),
            PathBuf::from("./test_set_paths/Documents")
        );
        let expected =
            format!("{}{}{}{}", DB_PREFIX, dir, "Documents/", DB_FNAME);
        assert_eq!(get_db_path(), expected);
    }
}
