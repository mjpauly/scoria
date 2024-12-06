//! Logic for mounted databases in addition to the main database.
//!
//! Currently can only view location tracks, not pins, in mounted dbs.
//!
//! Derived state stores mounted ids, which can be read by frontend.
//! If a mounted id exists in derived state which is not in frontend state, a
//! new frontend record is created which user can name and enable/disable.
//!
//! If a id exists in frontend state that is not in derived state, then indicate
//! it's not found on disk and have option to delete the frontend record
//!
//! When deleting, send msg to backend say to delete, which corresponds to a
//! return message saying db deleted, where frontend record gets deleted.

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, os::unix::prelude::PermissionsExt};
use tracing::error;

use common::mounted::{MountID, MAIN_DB_MOUNT_ID};

use crate::app_state::{
    get_derived_state, get_front_state, set_derived_state, AppState,
};
use crate::paths::{get_mount_root_dir, DB_FNAME};
use crate::tz::get_system_tz;
use crate::ws_session;

use super::open_db;

/// Open connections to all databases currently mounted on disk, excluding the
/// main database. This is run on foregrounding, since the mounted databases are
/// just for viewing, not logging new data.
pub async fn open_all_mounted() {
    if let Err(e) = try_open_all_mounted().await.context("init mounted dbs") {
        error!("{e:?}");
    }
}

/// Fallible routine for opening connections to each database.
async fn try_open_all_mounted() -> Result<()> {
    // update backend state with connections to all mounted dbs, and derived
    // state with list of all dbs on disk and any errors if applicable
    let mut db_connections = BTreeMap::new();
    let mut dbs_on_disk = BTreeMap::new();
    // copy over the main database, then fully overwrite the state with what we
    // find on disk
    if let Some(conn) = AppState::global()
        .dbs
        .lock()
        .unwrap()
        .get(&MAIN_DB_MOUNT_ID)
        .cloned()
    {
        db_connections.insert(MAIN_DB_MOUNT_ID, conn);
    }
    if let Some(msg) = get_derived_state(|s| {
        s.mounted_dbs_on_disk.get(&MAIN_DB_MOUNT_ID).cloned()
    }) {
        dbs_on_disk.insert(MAIN_DB_MOUNT_ID, msg);
    }
    let mount_root_dir = get_mount_root_dir();
    // might already exist, skip inspecting error
    let _ = fs::create_dir(&mount_root_dir);
    let entries =
        fs::read_dir(&mount_root_dir).context("reading mount root dir")?;
    for entry in entries {
        let entry = match entry.context("reading dir entry") {
            Ok(entry) => entry,
            Err(e) => {
                error!("{e:?}");
                continue;
            }
        };
        let db_id = entry.file_name();
        let db_id_str = db_id.to_string_lossy();
        let mount_dir = entry.path();
        let Ok(id) = db_id_str
            .parse::<MountID>()
            .context("parsing dir name as mount id")
            .map_err(|e| error!("{e:?}"))
        else {
            // can't parse as id -> remove unknown directory
            remove_mount_dir(&mount_dir);
            continue;
        };
        let maybe_conn = try_open_db(&mount_dir)
            .await
            .context("error opening mounted database");
        match maybe_conn {
            Ok(conn) => {
                db_connections.insert(id, conn);
                dbs_on_disk.insert(id, None);
            }
            Err(e) => {
                error!("{e:?}");
                dbs_on_disk.insert(id, Some(format!("{e:?}")));
            }
        };
    }
    *AppState::global().dbs.lock().unwrap() = db_connections;
    set_derived_state(|s| s.mounted_dbs_on_disk = dbs_on_disk);
    Ok(())
}

/// On backgrounding, close connections to all mounted databases.
pub async fn close_all_mounted() {
    if let Err(e) = try_close_all_mounted().await.context("close mounted dbs") {
        error!("{e:?}");
    }
}

/// Close all mounted database connections (excludes main database). Derived
/// state is only updated on foregrounding.
async fn try_close_all_mounted() -> Result<()> {
    let mut connections_to_close = Vec::new();
    {
        let app_state = AppState::global();
        let mut connections = app_state.dbs.lock().unwrap();
        let ids_to_remove = connections
            .keys()
            .filter(|id| **id != MAIN_DB_MOUNT_ID)
            .cloned()
            .collect::<Vec<_>>();
        for id in ids_to_remove.iter() {
            connections_to_close.push(connections.remove(id));
        }
    }
    for conn in connections_to_close.into_iter().flatten() {
        conn.close().await;
    }
    Ok(())
}

async fn try_open_db(mount_dir: &Path) -> Result<SqlitePool> {
    let db_path = mount_dir.join(DB_FNAME);
    let conn = open_db(db_path.to_string_lossy().to_string())
        .await
        .context("opening mounted db")?;
    Ok(conn)
}

/// Remove a mount directory, without any special error handling other than
/// logging. Used to cancel an already failing mount process.
fn remove_mount_dir(mount_dir: &Path) {
    if let Err(e) = fs::remove_dir_all(mount_dir).context("removing mount dir")
    {
        error!("{e:?}");
    }
}

/// Mount a database from a path.
pub async fn mount_db(path: PathBuf) {
    let (new_id, new_mount_dir) =
        match get_new_mount_dir().context("creating new db mount dir") {
            Ok(x) => x,
            Err(e) => {
                error!("{e:?}");
                ws_session::send_error_popup("failed to mount database");
                return;
            }
        };
    let conn = match copy_and_migrate_db(&path, &new_mount_dir).await {
        Ok(c) => c,
        Err(e) => {
            error!("mounting db: {e:?}");
            remove_mount_dir(&new_mount_dir);
            ws_session::send_error_popup("failed to mount database");
            return;
        }
    };
    let state = AppState::global();
    let mut mounted = state.dbs.lock().unwrap();
    mounted.insert(new_id, conn);
    let fname = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            // can't get filename -> use current timestamp
            let tz = get_system_tz().unwrap_or(jiff::tz::TimeZone::UTC);
            let now = jiff::Timestamp::now();
            let zdt = now.to_zoned(tz);
            zdt.to_string()
        });
    set_derived_state(|s| {
        s.last_mounted = Some((new_id, fname));
        s.mounted_dbs_on_disk.insert(new_id, None);
    })
}

/// Copy and migration of database. If this fails the mount is cancelled.
async fn copy_and_migrate_db(
    src_path: &PathBuf,
    new_mount_dir: &Path,
) -> Result<SqlitePool> {
    let new_path = new_mount_dir.join(DB_FNAME);
    fs::copy(src_path, &new_path).context("copying file")?;
    set_writable(&new_path).context("making file writable")?;
    let conn = open_db(new_path.to_string_lossy().to_string())
        .await
        .context("opening mounted db")?;
    Ok(conn)
}

/// fs::copy copies permission bits, so we need to make sure the file is
/// writable.
fn set_writable(path: &PathBuf) -> Result<()> {
    let f = fs::File::open(path)?;
    let metadata = f.metadata()?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o640);
    Ok(())
}

fn get_new_mount_dir() -> Result<(MountID, PathBuf)> {
    let mount_root_dir = get_mount_root_dir();
    fs::create_dir_all(&mount_root_dir)?;
    let entries =
        fs::read_dir(&mount_root_dir).context("reading mount root dir")?;
    // choose new id to be greater than the current largest to preserve id
    // ordering by import time
    let mut max_id = MAIN_DB_MOUNT_ID;
    for entry in entries {
        let id = entry
            .context("reading dir entry")?
            .file_name()
            .to_string_lossy()
            .parse::<MountID>()
            .context("parsing dir name as mount id")?;
        if id > max_id {
            max_id = id;
        }
    }
    let new_id = max_id + 1;
    let new_dir = mount_root_dir.join(new_id.to_string());
    fs::create_dir(&new_dir).context("creating new dir")?;
    Ok((new_id, new_dir))
}

pub async fn delete_mounted_db(id: MountID) {
    debug_assert!(id != MAIN_DB_MOUNT_ID); // should not be possible in ui
    let maybe_conn = {
        let state = AppState::global();
        let mut mounted = state.dbs.lock().unwrap();
        mounted.remove(&id)
    };
    set_derived_state(|s| s.mounted_dbs_on_disk.remove(&id));
    if let Some(conn) = maybe_conn {
        conn.close().await;
    }
    let mount_root_dir = get_mount_root_dir();
    let mount_dir = mount_root_dir.join(id.to_string());
    if let Err(e) = fs::remove_dir_all(mount_dir).context("removing mount dir")
    {
        error!("{e:?}");
        ws_session::send_error_popup("error deleting database");
    } else {
        ws_session::send_success_popup("Database deleted");
    }
}

/// Return the name for a database id if assigned, otherwise just the id. Used
/// for helpful error messages.
pub fn db_debug_name(id: &MountID) -> String {
    get_front_state(|s| s.mounted_db_settings.get(id).cloned())
        .flatten()
        .map(|setting| format!("{} ({})", setting.name, id))
        .unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
pub mod tests {
    use crate::app_state::{get_derived_state, AppState};
    use crate::database::mounted::open_all_mounted;
    use crate::database::open_db;
    use crate::init;
    use crate::local::local_fs_setup;

    #[tokio::test]
    async fn valid_db_on_disk_opens_without_error() {
        let dir = "valid_db_on_disk_opens_without_error/";
        let mount_id = 99;
        let paths = local_fs_setup(dir);
        let mount_dir =
            paths.documents_dir.join("mount").join(mount_id.to_string());
        std::fs::create_dir_all(&mount_dir).unwrap();
        let db_path = mount_dir.join("data.db").to_string_lossy().to_string();
        open_db(db_path).await.unwrap().close().await;
        init(paths, "1.test.0".into()).await;
        open_all_mounted().await;
        // should have a valid connection
        assert!(AppState::global()
            .dbs
            .lock()
            .unwrap()
            .get(&mount_id)
            .is_some());
        // and no errors
        assert!(get_derived_state(|s| s
            .mounted_dbs_on_disk
            .get(&mount_id)
            .unwrap()
            .is_none()));
    }

    #[tokio::test]
    async fn invalid_db_on_disk_opens_with_error() {
        let dir = "invalid_db_on_disk_opens_with_error/";
        let mount_id = 99;
        let paths = local_fs_setup(dir);
        let mount_dir =
            paths.documents_dir.join("mount").join(mount_id.to_string());
        std::fs::create_dir_all(&mount_dir).unwrap();
        let db_path = mount_dir.join("data.db");

        // create a text file and not a database
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&db_path)
            .unwrap();
        std::io::Write::write_all(&mut file, b"asdf").unwrap();

        init(paths, "1.test.0".into()).await;
        open_all_mounted().await;
        // no valid connection
        assert!(AppState::global()
            .dbs
            .lock()
            .unwrap()
            .get(&mount_id)
            .is_none());
        // and there an errors
        assert!(get_derived_state(|s| s
            .mounted_dbs_on_disk
            .get(&mount_id)
            .unwrap()
            .is_some()));
    }
}
